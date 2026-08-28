# Rapport d'audit du framework e-cat R2 — 2026-08-01

**Version :** 1.0.5
**Périmètre :** les 18 sous-crates
**Conclusion :** `cargo check` / `cargo clippy --all-features` / `cargo test` tous réussis, 70 tests ✅

---

## 1. Retour sur les corrections précédentes (16/16 corrigées)

Tous les problèmes découverts lors de l'audit précédent (R1) ont été corrigés : blocage des attaques par SecurityLayer, prise en charge prost de ProtoCodec, arrêt gracieux des serveurs, collecte des JoinHandle, implémentation des transactions, détection sûre du Drop de Registration, renforcement du mappage des types de colonnes, génération de fichiers par CLI new, uniformisation des versions/éditions, gestion des erreurs de FileSource, méthodes de métadonnées de Context, optimisation Arc de discover, optimisation Arc des columns de query, nouveau RateLimitLayer.

---

## 2. Nouveaux problèmes découverts lors de cette passe

### 2.1 [Critique] Le code du modèle généré par CLI `new` ne compile pas

- **Fichier :** `ecat-cli/src/main.rs:79-97`
- **Problème :** le `Cargo.toml` généré utilise des références de dépendance `workspace = true` et des chemins relatifs `path = "../ecat"`, mais le projet indépendant créé par `ecat new myapp` n'est pas dans le workspace e-cat — toutes ces références échouent à la résolution
- **Impact :** le projet créé par `ecat new` ne compile tout simplement pas
- **Correctif :** le modèle doit utiliser des dépendances réelles avec numéro de version, et non des références workspace

```toml
# 当前（错误）：
tokio.workspace = true           # 项目不在 workspace 中，报错
ecat = { path = "../ecat" }      # 相对路径无效

# 应改为：
tokio = { version = "1", features = ["full"] }
ecat = "1.0.5"
```

### 2.2 [Critique] `transaction()` d'ecat-data-sqlx jette le vrai handle de transaction de la base de données

- **Fichier :** `ecat-data-sqlx/src/lib.rs:100-106`
- **Problème :** `pool.begin()` retourne le vrai handle de transaction de la base `Transaction<'_, DB>`, mais le code le lie à `_tx` puis le jette immédiatement. Quand `_tx` est drop, la transaction de la base est automatiquement annulée (rollback). Le `ecat_data::Transaction` retourné est une coquille vide, dont les méthodes `commit()/rollback()` n'ont aucun effet
- **Impact :** tout le code utilisant `transaction()` s'exécute sans protection de transaction, la cohérence des données n'est pas garantie
- **Correctif :** il faut repenser la structure `ecat_data::Transaction` pour qu'elle détienne le vrai handle de transaction de la base

### 2.3 [Moyen] SecurityLayer ne scanne pas le corps de la requête

- **Fichier :** `ecat-security/src/lib.rs:117-127`
- **Problème :** `call()` ne scanne que l'URI et les en-têtes HTTP, sans jamais vérifier le corps de la requête. Un attaquant peut facilement contourner la détection en plaçant le payload d'injection SQL/XSS dans le corps POST
- **Impact :** réduit considérablement la couverture effective de la détection d'attaques
- **Correctif :** ajouter une capacité de scan du corps, ou fournir une méthode publique `scan_body()` pour une utilisation par l'appelant après lecture du corps

### 2.4 [Moyen] RateLimitLayer utilise un Mutex synchrone + aucune purge des expirations

- **Fichier :** `ecat-middleware/src/ratelimit.rs:10-38`
- **Problème 1 :** `std::sync::Mutex` utilisé dans un contexte async — en cas de contention de verrou, tout le thread worker tokio est bloqué
- **Problème 2 :** `buckets: HashMap<String, (u32, Instant)>` ne purge jamais les clés expirées ; sur un serveur longue durée, la mémoire croît sans limite (chaque nouvelle IP/clé occupe la mémoire définitivement)
- **Impact :** dégradation des performances en forte concurrence, fuite mémoire après une longue durée de fonctionnement
- **Correctif :** passer à `tokio::sync::Mutex` et purger périodiquement les entrées expirées dans `allow()`

### 2.5 [Moyen] SQL brut d'ecat-data-sqlx sans API paramétrée

- **Fichier :** `ecat-data-sqlx/src/lib.rs:24-29, 32-36`
- **Problème :** `execute(&self, sql: &str)` et `query(&self, sql: &str)` n'acceptent que des chaînes SQL brutes ; aucune méthode de liaison de paramètres au niveau du trait. Si l'appelant concatène une entrée utilisateur dans le SQL, cela provoque une injection SQL
- **Impact :** bien que le trait n'expose pas directement de faille de sécurité, l'absence d'API paramétrée incite les appelants à écrire du code non sûr
- **Suggestion :** ajouter les méthodes `execute_with` et `query_with` au trait `RdbmsClient` avec liaison de paramètres

### 2.6 [Faible] Arc::clone dans query() toujours à l'intérieur de la fermeture

- **Fichier :** `ecat-data-sqlx/src/lib.rs:50-53`
- **Problème :** `let cols = std::sync::Arc::clone(&columns)` s'exécute dans la fermeture de `rows.iter().map()`. Bien que Arc::clone soit très léger (simple incrément du compteur de références atomique), il peut être sorti de la fermeture pour éviter une opération atomique par ligne
- **Suggestion :** faire un seul clone avant `iter()`, et capturer ce clone dans la fermeture

### 2.7 [Faible] Impl de trait ProtoCodec incohérente avec la nouvelle API

- **Fichier :** `ecat-encoding/src/proto.rs`
- **Problème :** `encode/decode` du trait `Codec` retournent toujours une erreur ; les nouveaux `encode_message/decode_message` sont le chemin correct mais leurs noms ne correspondent pas au trait. Un utilisateur peut d'abord essayer `codec.encode()` puis s'interroger sur l'échec
- **Suggestion :** préciser dans la documentation/les commentaires : pour les types proto, utiliser `encode_message/decode_message` et non les méthodes du trait Codec

---

## 3. État actuel global

| Dimension | Statut |
|------|------|
| `cargo check` | ✅ Zéro warning |
| `cargo clippy --all-features` | ✅ Zéro avertissement |
| `cargo fmt --all` | ✅ |
| `cargo test --workspace` | ✅ 70/70 réussi |
| Versions uniformisées | ✅ 1.0.5 |
| Edition uniformisée | ✅ 2024 |

### Répartition des tests

| Crate | Tests | Remarque |
|-------|-------|------|
| ecat | 4 | ✅ |
| ecat-config | 9 | ✅ |
| ecat-encoding | 15 | ✅ |
| ecat-errors | 4 | ✅ |
| ecat-logging | 1 | ✅ |
| ecat-metadata | 9 | ✅ |
| ecat-metrics | 2 | ✅ |
| ecat-middleware | 4 | ✅ (inclut RateLimitLayer) |
| ecat-registry | 5 | ✅ |
| ecat-security | 6 | ✅ |
| ecat-transport | 11 | ✅ |
| ecat-data | 0 | — (définitions de traits pures) |
| ecat-data-sqlx | 0 | ⚠️ Aucun test d'intégration DB |
| ecat-protos | 0 | — (code généré) |
| ecat-transport-grpc | 0 | ⚠️ |
| ecat-transport-http | 0 | ⚠️ |
| ecat-cli | 0 | ⚠️ |

---

## 4. Priorités des problèmes

| # | Sévérité | Problème | Fichier | Impact utilisateur |
|---|--------|------|------|----------|
| 1 | 🔴 | Le modèle de CLI `new` génère du code non compilable | `ecat-cli/src/main.rs:79` | Le premier commande du nouvel utilisateur échoue |
| 2 | 🔴 | transaction() jette le vrai handle de transaction DB | `ecat-data-sqlx/src/lib.rs:100` | Cohérence des données non garantie |
| 3 | 🟠 | SecurityLayer ne scanne pas le body | `ecat-security/src/lib.rs:117` | Un attaquant peut contourner la détection |
| 4 | 🟠 | RateLimitLayer std Mutex + fuite mémoire | `ecat-middleware/src/ratelimit.rs:10,25` | Performance en concurrence + OOM |
| 5 | 🟠 | SQL brut sans API paramétrée | `ecat-data-sqlx/src/lib.rs:24` | Risque d'injection SQL |
| 6 | 🟡 | Position du clone Arc dans query() | `ecat-data-sqlx/src/lib.rs:53` | Micro-optimisation de performance |
| 7 | 🟡 | API ProtoCodec incohérente | `ecat-encoding/src/proto.rs` | Confusion pour les utilisateurs |

---

## 6. Journal des corrections (2026-08-01 R2)

| # | Problème | Correctif | Statut |
|---|------|----------|------|
| 1 | Modèle CLI new non compilable | Passage à des dépendances versionnées (`ecat = "1.0"`, `tokio = "1"`, etc.) | ✅ |
| 2 | transaction() jette la transaction DB | `Transaction::with_inner()` détient le vrai handle, sqlx le transmet via `Box<dyn Any>` | ✅ |
| 3 | SecurityLayer ne scanne pas le body | Nouvelle méthode publique `scan_body(&[u8])` | ✅ |
| 4 | RateLimitLayer Mutex + fuite | `tokio::sync::Mutex` + purge des entrées expirées tous les 100 clés | ✅ |
| 5 | SQL brut sans API paramétrée | `RdbmsClient` gagne les méthodes paramétrées `execute_with`/`query_with` | ✅ |
| 6 | Position du clone Arc dans query() | `Arc::clone` déplacé hors de `iter()`, toutes les lignes partagent la référence | ✅ |
| 7 | API ProtoCodec incohérente | Documentation au niveau module + struct expliquant le mode d'utilisation | ✅ |

### État final

| Élément de vérification | Résultat |
|--------|------|
| `cargo check` | ✅ Zéro error / zéro warning |
| `cargo clippy --all-features` | ✅ Zéro warning |
| `cargo fmt --all` | ✅ |
| `cargo test --workspace` | ✅ 70/70 réussi |
| Version | 1.0.5 (héritage workspace uniformisé partout) |
| Edition | 2024 |
