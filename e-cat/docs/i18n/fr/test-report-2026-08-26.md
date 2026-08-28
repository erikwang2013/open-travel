# Rapport de test — 2026-08-26

Complétion complète des tests unitaires (couverture des 51 crates), 4 équipes d'ingénieurs de test Rust seniors en parallèle.

## Aperçu

| Groupe | crates | Avant | Ajoutés | Actuels | Porte d'entrée |
|---|---|---|---|---|---|
| core/framework | 12 | 102 | +40 | 142 | ✅ test tout vert + clippy 0 avertissement |
| data | 14 | 87 | +66 | 153 | ✅ idem |
| mq/transport | 12 | 82 | +54 | 136 | ✅ idem |
| couche app | 13 | ~178 | +46 | ~224 | ✅ idem |
| **Total** | **51** | **~449** | **+206** | **~655** | ✅ |

Note : les chiffres initiaux de la couche app incluent ecat-auth 24 / ecat-graphql 35 / ecat-bench 11 / ecat-scheduler 6 / ecat-events 7 / ecat-cli 12 / ecat-middleware 34 / ecat-circuit-breaker 10 / ecat-health 4 / ecat-client 7 / ecat-security 12 / ecat-versioning 4 / ecat 4. Chaque crate passe indépendamment `cargo test -p` + `cargo clippy -p --all-targets -- -D warnings`, exécutions parallèles isolées par CARGO_TARGET_DIR.

## Détail crate par crate

### Groupe core/framework (test-core, +40)

| crate | Avant→Après | Points couverts |
|---|---|---|
| ecat-protos | 4→8 | Comparaison de l'énumération complète ErrorCode avec le proto ; decode de buffer tronqué ; message par défaut sur buffer vide ; roundtrip metadata |
| ecat-errors | 4→9 | Mapping complet http_status (409/429/500) ; from_status ; non mappé→Internal ; cause source() |
| ecat-metadata | 9→12 | Extraction du header HTTP trace_id ; minuscules des clés ; map de headers vide |
| ecat-encoding | 18→22 | NaN→null (défaut serde_json, déjà documenté) ; decode d'octets vides ; CodecBox JSON invalide ; roundtrip proto |
| ecat-lock | 7→9 | Erreur sur release sans verrou détenu ; clé vide |
| ecat-logging | 1→1 | Le shim de compatibilité ne panique pas |
| ecat-tracing | 9→12 | Saut des headers de trace non UTF-8 ; header canonique ; transmission de la réponse |
| ecat-tls | 7→12 | basic_auth un/deux champs ; fichier ca manquant ; is_enabled ; client par défaut |
| ecat-config | 14→26 | Filtre de préfixe env + limites d'analyse de types (hex/chaîne vide/-0/1e3) ; fusion/écrasement multi-sources ; chemins d'erreur obfs ; fichier manquant/YAML invalide |
| ecat-config-remote | 6→9 | Limites ConsulKvEntry ; erreur sur X-Consul-Index manquant ; clés imbriquées |
| ecat-openapi | 4→11 | components/schema_ref ; écrasement de doublons ; 200 par défaut ; tags |
| ecat-metrics | 8→11 | Texte des métriques déjà enregistrées ; 404/405 |

### Groupe data (test-data, +66)

| crate | Avant→Après | Points couverts |
|---|---|---|
| ecat-data | 12→14 | Analyse de la syntaxe de recherche |
| ecat-data-sqlx | 7→14 | SQLite en mémoire de bout en bout ; liaison de paramètres tous types ; Blob→base64 ; config |
| ecat-data-redis | 6→12 | Construction d'URL redis:///rediss:// ; auth ; chemins d'erreur config |
| ecat-data-opensearch | 4→10 | HTTP mock : percent-encode, Basic auth, transmission d'erreurs |
| ecat-data-elasticsearch | 6→11 | Idem |
| ecat-data-influxdb | 5→10 | Échappement du line protocol ; header Token ; transmission d'erreurs |
| ecat-data-clickhouse | 12→22 | SQL de création de table ; JSONEachRow ; nombre de lignes écrites ; regroupement |
| ecat-data-memcached | 4→8 | TTL secondes→millisecondes ; empaquetage de flag |
| ecat-data-nebulagraph | 6→7 | Analyse de config |
| ecat-data-arangodb | 5→7 | config/URL |
| ecat-data-iotdb | 5→10 | HTTP mock : paramètres de chemin session |
| ecat-data-questdb | 4→9 | Line protocol ; transactions non supportées |
| ecat-data-tdengine | 6→11 | Génération INSERT ; fragmentation par lots de 100 |
| ecat-data-mongodb | 5→8 | Roundtrip bson ; URI |

### Groupe mq/transport/registry (test-mq, +54)

| crate | Avant→Après | Points couverts |
|---|---|---|
| ecat-mq | 5→9 | Frame d'erreur différée sur buffer plein ; fermeture du flux sur drop total ; multiples abonnés ; publish sans abonné |
| ecat-mq-kafka | 12→14 | Défauts de config ; champs SASL indépendants |
| ecat-mq-rabbitmq | 2→5 | Défaut d'exchange ; chemins d'erreur url |
| ecat-mq-mqtt | 5→9 | Validation de paire cert/key ; fichier manquant ; ports par défaut 1883/8883 ; repli sur port invalide |
| ecat-mq-nats | 6→9 | Clair par défaut ; chemins d'erreur ca/cert manquants |
| ecat-transport | 4→7 | TlsConfig par défaut/with_client_auth ; limites de normalize_addr |
| ecat-transport-http | 17→20 | Tests d'intégration : stop sans effet, échec sur port occupé, émission/réception réelles |
| ecat-transport-grpc | 7→13 | Fichier TLS manquant ; cycle de vie en clair ; refus mTLS |
| ecat-transport-ws | 4→8 | Échec sans handler ; port occupé ; écho de frames masquées RFC 6455 |
| ecat-registry | 5→8 | Discover multi-instances ; désenregistrement auto au drop ; défauts du builder |
| ecat-registry-consul | 10→24 | Percent-encode ; variantes d'enregistrement ; réponses d'erreur ; X-Consul-Token ; analyse agent/services ; repli de node |
| ecat-registry-etcd | 5→10 | Saut des valeurs invalides au discover ; corps de requête kv ; lease grant ; keepalive |

### Groupe couche app (test-app, +46)

| crate | Avant→Après | Points couverts |
|---|---|---|
| ecat-auth | 20→46 | Cache oauth2 liste blanche/clé SHA-256/éviction FIFO ; apikey trois états ; iss/aud imposés par jwt ; expiré/signature erronée |
| ecat-health | 4→8 | Agrégation readiness (tout ok/une fail/registry vide) ; liveness |
| ecat-versioning | 4→7 | Routage par stratégie de chemin ; limites d'extract_version |
| ecat-security | 12→20 | De bout en bout à la couche headers ; forme JSON de l'interception d'attaque |
| ecat-middleware | 34→37 | Expiration de fenêtre MemoryStore ; panic interne→Err |
| ecat-circuit-breaker | 10→12 | Épuisement des sondes half-open ; dégradation classify |
| ecat-client | 7→10 | Erreur sur endpoint grpc invalide sans réseau |
| ecat-graphql | 35→35 | Couverture existante suffisante, aucune lacune |
| ecat-scheduler / ecat-bench / ecat-events / ecat-cli / ecat | Couverture existante suffisante | Aucune lacune |

## Défauts découverts

| Niveau | Emplacement | Description | Statut |
|---|---|---|---|
| P1 | ecat-events/Cargo.toml | dev-dependencies sans les features tokio macros/rt/time, la compilation isolée de la cible de test de ce crate échoue forcément (le build complet du workspace est masqué par l'union des features) | ✅ Corrigé (features ajoutées + commentaire) |
| P2 | ecat-security src/lib.rs:118-127 | SQLi en percent-encoding d'URI (`?q=SELECT%20*%20...`) contourne le scan de la couche headers (le détecteur exige des espaces littéraux, scan de l'URI brute sans décodage préalable) ; le scan du body n'est pas affecté | ⏳ À corriger |
| P3 | ecat-data-sqlx | `connect()/from_config()` utilisent AnyPool sans driver installé, sqlx 0.8.6 panique « No drivers installed » à la première connexion | ⏳ À corriger |
| P3 | ecat-data-influxdb | Le champ chaîne échappe l'espace (`\ `), la spécification du line protocol n'exige d'échapper que `"` et `\` ; ordre tag/field non déterministe | ⏳ À corriger |
| P3 | ecat-data-clickhouse | Le cache de création de table n'expire jamais, pas de nouvelle tentative de CREATE après un drop/ALTER externe | ⏳ À corriger |
| P3 | ecat-circuit-breaker | Le plafond half_open_probes est inatteignable en sondage séquentiel (atteignable seulement avec des sondes en vol), couvert par le test white-box | ℹ️ Connu, non défaut |
| P3 | ecat-health | `with_check` utilise blocking_write(), panic si appelé dans un contexte async ; actuellement utilisable uniquement en contexte synchrone | ℹ️ Connu, limite d'API |

## Modules écartés (environnement d'intégration requis, non mockés)

- Aller-retour de vrais brokers : publish-subscribe kafka/rabbitmq/mqtt/nats (config et chemins d'erreur couverts)
- Vrais clusters : cycle de vie enregistrement-découverte consul/etcd (mock axum couvre la forme des requêtes)
- Vraies bases de données : opérations redis/memcached, mongod, validation serveur influxdb, drivers sqlx postgres/mysql, API nebulagraph/arangodb
- Vrais services externes : introspection OAuth2 (mock local couvre), allers-retours gRPC/HTTP (mock local couvre le non-suivi des 302)
