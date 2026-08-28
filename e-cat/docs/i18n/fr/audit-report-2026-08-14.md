# Rapport d'audit spécialisé (sécurité et performance) — 2026-08-14

Périmètre de l'audit : workspace de 55 crates (v2.3.5). Méthode : vérification manuelle de Cargo.lock (cargo-audit non installé), audit des chemins d'authentification/TLS dans le source, vérification de la concurrence et du cycle de vie des ressources. Aucun code soumis.

## Vérification des CVE des dépendances

- Les versions des dépendances de base sont récentes et sans CVE connue non corrigée : rustls 0.23.43, ring 0.17.14, aws-lc-rs 1.17.3, jsonwebtoken 9.3.1, tokio 1.53.1, h2 0.4.15, quinn 0.11.11, sqlx 0.8.6, zerocopy 0.8.55, time 0.3.54, openssl 0.10.81.
- hyper 0.14.32 (uniquement via rust-s3 0.35.1, par hyper-tls 0.5) est au-dessus de la ligne de correctif 0.14.28.
- À noter : le CI n'installe pas cargo-audit, il est suggéré d'ajouter cette vérification automatisée au workflow.

## Découvertes (triées par gravité)

### S1 [Moyen] Sérialisation de la poignée de main TLS HTTP → DoS par poignées de main lentes
- Emplacement : `ecat-transport-http/src/lib.rs:134-150` (TlsListener::accept)
- Phénomène : la poignée de main TLS s'effectue en synchrone dans `accept()`, axum::serve appelle accept en série — une connexion qui ne termine pas sa poignée de main bloque toute la boucle d'accept.
- Impact : un attaquant ouvrant en masse des connexions TCP lentes/mortes peut faire cesser totalement l'acceptation de nouvelles connexions (côté gRPC, tonic spawn la poignée de main par connexion, non affecté).
- Suggestion : après accept, `tokio::spawn` la poignée de main avec `tokio::time::timeout(10s)`, fermer la connexion en cas d'échec.

### S2 [Moyen] Croissance illimitée du cache d'introspection OAuth2 → DoS mémoire
- Emplacement : `ecat-auth/src/oauth2.rs:45,84-92`
- Phénomène : `HashMap<String,(String,Instant)>` indexé par token, le TTL ne contrôle que la fraîcheur, sans limite de capacité ni éviction.
- Impact : un flot de tokens uniques peut faire croître la mémoire sans limite (chaque miss déclenche aussi une introspection en amont).
- Suggestion : ajouter une limite de capacité (10k par ex.) + nettoyage périodique, ou passer à moka/LRU avec éviction par capacité et TTL.

### S3 [Faible-moyen] ecat-data-s3 utilise l'ancien rust-s3 0.35.1 (hyper 0.14 + native-tls/openssl)
- Emplacement : `ecat-data-s3/Cargo.toml` → rust-s3 0.35.1
- Phénomène : le client S3 utilise indépendamment la pile hyper-tls/openssl, `ecat-tls::TlsClientConfig` (CA personnalisé, certificat client, skip_verify) est sans effet sur S3 ; surface de configuration TLS incohérente.
- Impact : la CA privée/mTLS S3 en environnement d'entreprise n'est pas configurable ; dépendance peu maintenue depuis 2023.
- Suggestion : évaluer la mise à niveau de rust-s3 ou passer à un client unifié reqwest/rustls.

### S4 [Faible] La validation JWT par défaut n'inclut pas iss/aud
- Emplacement : `ecat-auth/src/jwt.rs:125` — `Validation::new(HS256)` ne vérifie que signature + exp.
- Impact : avec une clé partagée HS256, le token d'un service peut être accepté par un autre service (pas d'isolation par émetteur).
- Suggestion : la documentation exige explicitement de configurer issuer/audience en production ; ou ajouter un point d'entrée de validation iss par défaut.

### S5 [Faible] `TlsClientConfig.skip_verify` seul rend is_enabled() vrai
- Emplacement : `ecat-tls/src/lib.rs:23-29`
- Phénomène : avec uniquement `skip_verify: true`, TLS est considéré « activé » sans vérification de certificat, la validation est désactivée silencieusement.
- Suggestion : validation mutuellement exclusive de skip_verify et ca_cert, ou exiger une double confirmation explicite.

## Performance et ressources

### P1 [Faible] Désérialisation JSON par requête sur le chemin de hit du cache OAuth2
- Emplacement : `ecat-auth/src/oauth2.rs:87` — le cache stocke une chaîne sérialisée, `serde_json::from_str` est encore appelé après un hit.
- Suggestion : stocker directement la structure `AuthClaims` dans le cache, économisant le parse par requête.

### P2 [Faible] ecat-bench sans préchauffage ni jugement d'état stationnaire
- Emplacement : `ecat-bench/src/lib.rs:run_bench` — chronométrage direct, sans warmup ; le démarrage à froid/l'alloc initiale du pool de connexions se mêlent au p99.
- Suggestion : ajouter des tours de préchauffage et un critère de convergence vers l'état stationnaire pour des résultats plus fiables.

### P3 [Faible] Kafka consommateur : 100 ms poll + 100 ms sleep en série
- Emplacement : `ecat-mq-kafka/src/lib.rs:84-92` — latence de bout en bout du message plafonnée à ~200 ms.
- Suggestion : plus besoin de sleep après poll ; en faible débit, réduire l'intervalle de poll.

## Confirmation des bonnes pratiques

- Aucun unwrap/expect panic dans les chemins de production (transport/auth/middleware uniquement dans les tests).
- Le repli sur paramètre de requête pour la clé API émet un avertissement de fuite dans les logs ; HashMap utilise SipHash contre les collisions.
- La couche SQL transmet le SQL de l'appelant (nature de framework), l'encodage pourcentage user:pass dans la chaîne de connexion est correct.
- Canal de consommation Kafka plein : backpressure bloquante plutôt que perte ; après drop de rx, la tâche de poll sort normalement.
- Le pull de config-remote est muni de timeouts (5s/30s), les requêtes bloquantes sans index signalent une erreur contre l'attente active.

---

## Audit de correction du domaine central (supplément, complémentaire aux spécialités sécurité/performance ci-dessus)

Méthode d'audit : scan du code de production de tout le workspace (localisation unwrap/expect/panic, erreurs avalées silencieusement, arrêt asynchrone, état concurrent) + revérification complète `cargo test --workspace` (premier tour tout vert ; le correctif S1 en cours a provoqué des avertissements de compilation intermédiaires dans transport-http, à relancer après finalisation). Aucun code soumis.

### N1 [Moyen] Fuite de handle après sortie de la tâche de consommation ecat-events → perte d'événements silencieuse
- Emplacement : `ecat-events/src/lib.rs:97-101` (boucle de consommation lignes 89-95 `None => break`)
- Phénomène : quand le stream mq renvoie None (par ex. fermeture du canal broadcast kafka) ou que la tâche panique, la boucle de consommation sort mais les JoinHandle restent dans la map `consumers` ; ensuite, un `subscribe()` du même type d'événement ne redémarre plus la tâche de consommation (ligne 68 `contains_key` toujours vrai) → perte silencieuse permanente de ce type d'événement.
- Impact : après interruption du flux d'événements distant, pas d'auto-guérison ; la récupération exige un redémarrage du processus.
- Suggestion : retirer le handle de la map dans le chemin de sortie de la tâche (spawn d'un watcher ou nettoyage paresseux par `handle.is_finished()`).

### N2 [Moyen] Sémantique erronée de group_id dans le subscribe d'ecat-mq-kafka
- Emplacement : `ecat-mq-kafka/src/lib.rs:71-84`
- a. `group_id` None par défaut : `consumer.subscribe()` de rdkafka exige group.id (librdkafka renvoie INVALID_ARG), l'abonnement par défaut échoue probablement directement (à vérifier sur machine réelle).
- b. Avec group_id configuré (ecat-events subscribe une fois par type d'événement, même group), Kafka répartit le topic par partitions entre les consommateurs du même groupe → un type d'événement peut tomber sur une tâche de consommation d'un autre type et être silencieusement abandonné (auto.offset.reset=latest et aucun commit).
- Impact : le bus d'événements perd des événements sur le backend kafka.
- Suggestion : générer un group.id aléatoire unique en l'absence de group_id ; ou utiliser assign() côté consommation pour l'allocation explicite des partitions ; la documentation précise que les multi-abonnements doivent avoir des groupes indépendants.

### N3 [Faible] Host vide non normalisé dans GrpcServer/WsServer (correctif D1 incomplet)
- Emplacement : `ecat-transport-grpc/src/lib.rs:52`, `ecat-transport-ws/src/lib.rs:58`
- Phénomène : `addr.parse::<SocketAddr>()` de `GrpcServer::new(":8000")` renvoie AddrParseError (vérifié empiriquement) ; `TcpListener::bind(":8000")` de WsServer résout vers le joker IPv6, échec au démarrage sans IPv6. HttpServer a déjà sa normalisation 0.0.0.0, les trois API de serveur se comportent différemment.
- Suggestion : normaliser uniformément le host vide dans new.

### N4 [Faible] TracingLayer n'injecte pas de trace_id, contraire à la déclaration du CHANGELOG 2.3.3
- Emplacement : `ecat-tracing/src/lib.rs:72-84` (le span ne contient que le champ service, le commentaire de code reconnaît qu'un Req générique ne permet pas de prendre les headers) ; `inject_trace_id()` génère un nouvel UUID à chaque appel, sans reprendre le trace_id extrait en amont.
- Impact : le traçage distribué configuré selon la documentation ne peut pas relier les services entre eux.
- Suggestion : liaison différée des champs du span ou spécialisation http::Request<B> ; inject supporte le transport de l'id amont.

### N5 [Faible] Panic d'un job ecat-scheduler → arrêt silencieux
- Emplacement : `ecat-scheduler/src/lib.rs:53-57,83` (`let _ = handle.await` dans `run()`)
- Phénomène : après un panic, la tâche planifiée meurt sans redémarrage ni log ; `run()` abandonne l'erreur du JoinHandle.
- Suggestion : capturer le panic avec log + stratégie de redémarrage optionnelle.

### N6 [Faible] unwrap résiduels dans le code de production (chemins d'empoisonnement/panic)
- `ecat-events/src/lib.rs:68,98` `Mutex::lock().unwrap()` std (panic si empoisonné) ; `ecat-versioning/src/lib.rs:86` unwrap du builder de Response (infaillible mais chemin de panic) ; `ecat-mq/src/lib.rs:110` expect protégé par le garde is_none (sûr).
- Suggestion : les deux endroits d'events passent à `unwrap_or_else(|e| e.into_inner())`.

### N7 [Information] WsServer::stop() n'attend pas les connexions WebSocket déjà mises à niveau
- Emplacement : `ecat-transport-ws/src/lib.rs:63-87`
- Les connexions axum on_upgrade s'exécutent dans des tâches indépendantes, non couvertes par l'arrêt gracieux ; les handlers de longues connexions restent après stop(), le processus ne sort pas proprement (sémantique App::stop incomplète).

### N8 [Information] Crates à zéro test : ecat-data / ecat-lock / ecat-protos
- Tous des crates de traits/définitions ; les méthodes par défaut ont été vérifiées fail-loud (erreur renvoyée plutôt que silencieux), mais les contrats de traits (sémantique de rollback au drop de Transaction, validation de token de verrou) n'ont aucun test unitaire.
- Suggestion : ajouter des tests unitaires minimaux pour les sémantiques de RdbmsError/Transaction et DistributedLock.

### N9 [Information] Les paramètres graphql et les champs imbriqués sont toujours abandonnés
- `ecat-graphql/src/lib.rs` : execute ne transmet que `variables` au resolver, les paramètres de champ de `{ hello(name: "x") }` et les selections imbriquées ne sont pas transmis ; le README ne mentionne pas cette limite (l'ancien rapport L8 demandait la documentation, toujours pas complétée après la réécriture 2.3.3).

### N10 [Information] circuit-breaker ne compte que les erreurs de la couche transport
- `ecat-circuit-breaker/src/lib.rs:203-209` ne compte comme échec que l'Err interne, les 5xx HTTP sont considérées comme succès → le fusible est inefficace contre l'indisponibilité du service (tempête de 5xx) ; la documentation ne le précise pas.

**État de vérification :** premier tour `cargo test --workspace` tout vert (y compris les doc-tests, aucune échec en fin de sortie) ; pendant l'édition du correctif S1, transport-http a montré des erreurs de compilation et 2 avertissements (import inutilisé `ensure_crypto_provider`, `shutdown_tx` non lu) — état intermédiaire, après finalisation de S1 il faut relancer intégralement les tests et `clippy --all-targets -D warnings`.

---

## Troisième passe : validation dynamique + re-vérification CVE + surface de panic (spécialisée, 2026-08-14)

### Re-vérification CVE (nouvelles découvertes, par gravité)

1. **[Moyen] rustls-webpki 0.102.8 résiduel dans l'arbre de dépendances** (RUSTSEC-2026-0049/0098/0099/0104 : contournement du distributionPoint CRL, name-constraints URI/wildcard, version corrigée 0.103.10). La chaîne principale est en 0.103.13 (via rustls 0.23.43, sûre) ; 0.102.8 est introduit par async-nats 0.38.0 / rumqttc 0.25.1, couvrant les chaînes clientes TLS NATS/MQTT. L'amont n'a pas migré vers rustls 0.23, aucune version corrigée — risque maîtrisé, suivi par commentaire suggéré.
2. **[Moyen-faible] rdkafka 0.36.2 embarque librdkafka avec cJSON 1.7.14** (CVE-2023-53154 et la série cJSON ; CVE-2025-57052 marquée CVSS 9.8 mais le fichier affecté cJSON_utils.c n'est pas utilisé par librdkafka, applicabilité douteuse). Le correctif amont est dans librdkafka 2.10+ (PR #5346, 2026-03). ecat-mq-kafka lie statiquement, il faut vérifier la version empaquetée de librdkafka-sys et suivre la mise à niveau.
3. **[Faible] rustls-pemfile 2.2.0 non maintenu** (RUSTSEC-2025-0134) — ecat-transport-http analyse des fichiers locaux au démarrage, ce ne sont pas des entrées d'attaquant.
4. **[Faible] rsa 0.9.10** (RUSTSEC-2023-0071 canal auxiliaire de temporisation Marvin) — introduit par le TLS de sqlx-mysql, pertinent uniquement dans le scénario MySQL + échange de clés RSA.
5. async-nats 0.38.0 est au-dessus de la ligne de correctif de RUSTSEC-2023-0027 (contournement de vérification CN), sans problème.

### Validation dynamique (examples/helloworld, build debug, port temporaire 18080, nettoyé)

- /health 200, / (sérialisation JSON) 200 (27B), 404 normal ; le middleware Logging enregistre normalement les requêtes.
- **/metrics monté mais renvoie 200 + body vide (0 octet)** : sans enregistrement de métriques, aucune sortie ; le côté supervision ne peut pas distinguer « sain/pas de métriques ». Suggéré : une ligne de commentaire dans le registry vide ou 503.
- Requête malformée (headers contenant 0x01/0x02) → 400 Bad Request, le service reste vivant, /health suivant est toujours 200, aucun panic.
- Chemins TLS/mTLS et middlewares fusible/limitation : couverts par les tests de ecat-transport-http/grpc, ecat-middleware (tout vert après le correctif de la course mTLS, les cas de refus des certificats anonymes/incorrects passent).

### Baseline bench

- ecat-bench n'a pas de cible [[bench]]/bin, pas de point d'entrée cargo bench ; run_bench_with_warmup est muni du préchauffage (correctif P2 déployé), les tests du harness sont tout verts.
- Mesure réelle en smoke de build debug : / ~1,3 ms, /health ~1,8 ms (coût du processus curl inclus, sans valeur de baseline). Suggéré : build release + charge wrk/hey pour une vraie baseline.

### Re-vérification de la surface de panic (tout le workspace, modules de test exclus)

- 31 unwrap/expect/panic au total, tous à faible risque : `Response::builder().body().unwrap()` (branches infaillibles de jwt/apikey/oauth2), filets d'empoisonnement de verrou (etcd/testing), `serde_json::to_string().unwrap()` de clickhouse (panic théorique sur entrée NaN/inf extrême).
- **1 à surveiller :** `ecat-transport-http/src/tls_listener.rs:234` — panic! dans `accept()` quand la boucle d'accept en arrière-plan sort anormalement, le thread de service meurt (conditions de déclenchement rares : seule une erreur fatale du listener) ; suggéré de le dégrader en retour d'erreur avec log.
