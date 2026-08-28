# Rapport d'audit E-CAT — r5

**Date :** 2026-08-01  
**Branche :** main  
**Version :** 2.1.7  
**Nombre de crates :** 47 (membres du workspace)
**Statut :** ✅ tous les problèmes corrigeables résolus + prise en charge complète des fichiers de configuration pour les backends de données

---

## 0. Journal des corrections (2026-08-01)

| # | Problème | Fichier | Correctif |
|---|------|------|------|
| 1 | import inutilisé `axum::routing::get` | `ecat-versioning/src/lib.rs:3` | Suppression de l'import de niveau supérieur, déplacé dans `#[cfg(test)]` |
| 2 | variable inutilisée `version` | `ecat-versioning/src/lib.rs:61` | Renommée `_version` |
| 3 | code mort `extract_version` | `ecat-versioning/src/lib.rs:68` | Passée en `pub fn` |
| 4 | `useless_format!` | `ecat-versioning/src/lib.rs:62` | Remplacement par `"/api"` direct |
| 5 | `unnecessary_to_owned` | `ecat-data-questdb/src/lib.rs:39` | `"true".to_string()` → `"true"` |
| 6 | Message d'erreur avalé | `ecat-data-questdb/src/lib.rs:30` | `unwrap_or_default()` → `unwrap_or_else(...)` |
| 7 | `derivable_impls` | `ecat-client/src/lib.rs:249` | `GrpcClientBuilder` passe à `#[derive(Default)]` |
| 8 | `manual_is_multiple_of` | `ecat-config/src/encrypted.rs:60` | `s.len() % 2 != 0` → `!s.len().is_multiple_of(2)` |
| 9 | `collapsible_if` | `ecat-registry-etcd/src/lib.rs:92` | Fusion des `if let` imbriqués |
| 10 | `collapsible_if` | `ecat-data-clickhouse/src/lib.rs:56` | Fusion des `if let` imbriqués |
| 11 | `type_complexity` | `ecat-data-memcached/src/lib.rs:9` | Ajout de l'alias de type `CacheEntry` |

**Résultat final :** `cargo build` zéro warning, `cargo clippy --all-targets` zéro warning, `cargo test` tout réussi (0 échec).

### 12 ─ Prise en charge complète des fichiers de configuration pour les backends de données (Cargo + lib.rs)

Ajout de la structure `Config` (`#[derive(Deserialize)]`) et du constructeur `from_config()` pour 12 crates de backends de données, permettant de charger les informations de connexion depuis des fichiers de configuration JSON/YAML, sans codage en dur.

| Crate | Structure Config | Champs |
|-------|--------------|------|
| `ecat-data-redis` | `RedisConfig` | `url` |
| `ecat-data-sqlx` | `SqlxConfig` | `url` |
| `ecat-data-clickhouse` | `ClickhouseConfig` | `base_url`, `database` (défaut "default") |
| `ecat-data-questdb` | `QuestdbConfig` | `base_url` |
| `ecat-data-elasticsearch` | `ElasticsearchConfig` | `base_url` |
| `ecat-data-opensearch` | `OpenSearchConfig` | `base_url` |
| `ecat-data-influxdb` | `InfluxConfig` | `base_url`, `org`, `bucket`, `token` |
| `ecat-data-memcached` | `MemcachedConfig` | (vide — implémentation en mémoire) |
| `ecat-data-neo4j` | `Neo4jConfig` | `base_url`, `username`, `password` |
| `ecat-data-nebulagraph` | `NebulaGraphConfig` | `base_url`, `space` |
| `ecat-data-arangodb` | `ArangoConfig` | `base_url`, `db`, `username`, `password` |
| `ecat-data-iotdb` | `IotdbConfig` | `base_url`, `username`, `password` |

**Exemple d'utilisation :**
```rust
// 从 YAML 配置文件加载
let cfg: ClickhouseConfig = serde_json::from_str(r#"{"base_url":"http://localhost:8123"}"#)?;
let client = ClickhouseClient::from_config(cfg);
```

### 13 ─ Prise en charge d'authentification optionnelle pour les backends HTTP (5 crates)

Ajout des champs optionnels `username` / `password` et du constructeur `with_auth()` pour 5 backends purement HTTP. Tous en `Option<String>` (`#[serde(default)]`) ; sans configuration, pas d'authentification.

| Crate | Nouveaux champs Config | Nouveau constructeur |
|-------|-----------------|-------------|
| `ecat-data-elasticsearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-opensearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-clickhouse` | `username?`, `password?` | `with_auth()` |
| `ecat-data-questdb` | `username?`, `password?` | `with_auth()` |
| `ecat-data-nebulagraph` | `username?`, `password?` | `with_auth()` |

Toutes les requêtes HTTP attachent automatiquement le Basic Auth via la méthode auxiliaire `apply_auth()` (uniquement quand les deux ne sont pas None).

### 14 ─ Champs d'authentification optionnels pour Redis / RDBMS / Memcached (3 crates)

| Crate | Nouveaux champs Config | Nouveau constructeur | Mode d'authentification |
|-------|-----------------|-------------|----------|
| `ecat-data-redis` | `password?` | `connect_with_password()` | Mot de passe intégré à l'URL |
| `ecat-data-sqlx` | `username?`, `password?` | `connect_with_auth()` | Authentification intégrée à l'URL |
| `ecat-data-memcached` | `username?`, `password?` | `with_auth()` | Champs réservés (implémentation en mémoire) |

Sqlx couvre les quatre RDBMS SQLite / PostgreSQL / MySQL / TiDB. Les champs Auth sont intégrés à l'URL de connexion via `replacen("://", "://user:pass@")`, actif uniquement quand l'URL ne contient pas `@`.

### 15 ─ Prise en charge de l'authentification par certificat TLS + crate ecat-tls (les 12 backends)

Nouveau crate `ecat-tls`, fournissant :
- `TlsClientConfig` — configuration TLS optionnelle (ca_cert, client_cert, client_key, skip_verify)
- `generate_ca()` — génération de certificat CA auto-signé
- `generate_server_cert()` — génération de certificat serveur
- `generate_client_cert()` — génération de certificat client (mTLS)

Les Config des 12 backends de données gagnent tous le champ `#[serde(default)] tls: Option<TlsClientConfig>`.

| Type de backend | Mode TLS |
|----------|----------|
| 9 backends HTTP | `tls.build_reqwest_client()` construit le client reqwest TLS |
| Redis | Bascule du schéma d'URL `redis://` → `rediss://` |
| Sqlx | Champ réservé (TLS via paramètre d'URL `?sslmode=require`) |
| Memcached | Champ réservé (prévu pour l'implémentation réseau) |

---

## 1. Vue d'ensemble

| Élément | Statut | Détails |
|------|------|------|
| `cargo build` | ✅ Réussi | 3 warnings du compilateur, 19.85 s |
| `cargo test` | ✅ Réussi | ~137 tests unitaires tous réussis, 0 échec, 1 ignoré |
| `cargo clippy` | ⚠️ Avec warnings | 5 lint warnings répartis sur 3 crates |
| `cargo fmt` | ✅ Réussi | Aucun problème de format |
| `cargo audit` | ❌ Non installé | Impossible de scanner les CVE connues |

---

## 2. Warnings du compilateur (à corriger)

### 2.1 ecat-versioning (3 warnings)

**Fichier :** `ecat-versioning/src/lib.rs`

| # | Warning | Ligne | Sévérité |
|---|---------|------|----------|
| 1 | `unused import: axum::routing::get` | 3 | Faible |
| 2 | `unused variable: version` | 61 | Faible |
| 3 | `function extract_version is never used` | 68 | Faible |

**Suggestion :** supprimer l'import inutilisé, renommer `version` en `_version`, passer `extract_version` en `pub` ou la marquer `#[allow(dead_code)]`.

### 2.2 ecat-data-questdb (1 warning clippy)

**Fichier :** `ecat-data-questdb/src/lib.rs:39`

```rust
// 当前:
.query(&[("query", sql), ("count", &"true".to_string())])

// 应改为:
.query(&[("query", sql), ("count", &"true")])
```

### 2.3 ecat-client (1 warning clippy)

**Fichier :** `ecat-client/src/lib.rs:249`

`GrpcClientBuilder` implémente `Default` manuellement, il peut être remplacé directement par `#[derive(Default)]`.

---

## 3. Récapitulatif des warnings Clippy

| Crate | Warning | Type |
|-------|---------|------|
| ecat-versioning | `useless_format!` — utilise `"/api".to_string()` | Performance |
| ecat-versioning | import inutilisé / code mort | Nettoyage |
| ecat-data-questdb | `unnecessary_to_owned` | Performance |
| ecat-client | `derivable_impls` — utiliser derive Default | Simplification |

---

## 4. Analyse de la couverture des tests

### 4.1 Statistiques

| Indicateur | Valeur |
|------|------|
| Nombre total de tests unitaires | ~137 |
| Échecs | 0 |
| Ignorés | 1 |
| Crates avec tests | ~24 / 48 |
| **Crates à 0 test** | **~24 / 48 (50 %)** |

### 4.2 Crates manquant de tests (0 ou uniquement tests de construction)

Les crates suivants ont une couverture de test faible :

- ecat-data-arangodb, ecat-data-clickhouse, ecat-data-elasticsearch
- ecat-data-influxdb, ecat-data-iotdb, ecat-data-nebulagraph
- ecat-data-neo4j, ecat-data-opensearch, ecat-data-questdb
- ecat-data-redis, ecat-data-sqlx, ecat-data-memcached
- ecat-mq, ecat-mq-kafka, ecat-graphql, ecat-openapi
- ecat-transport, ecat-transport-grpc, ecat-transport-http
- ecat-transport-ws, ecat-tracing, ecat-logging
- ecat-middleware, ecat-registry-consul, ecat-registry-etcd

### 4.3 Doc-tests

Les doc-tests des **48 crates sont tous à 0**. Aucun exemple de documentation `/// ````rust` dans le code.

---

## 5. Problèmes de dépendances

### 5.1 ⚠️ yaml_serde vs serde_yaml (risque moyen)

**Fichier :** `ecat-config/Cargo.toml:9`

```toml
yaml_serde = "0.10"
```

La bibliothèque YAML standard de l'écosystème Rust est `serde_yaml` (dernière version `0.9.34+`), alors que `yaml_serde` est un crate **différent et moins maintenu**.

**Suggestion :** confirmer que `yaml_serde` est la dépendance voulue. Si l'intention était `serde_yaml`, remplacez-la.

### 5.2 cargo-audit manquant

`cargo audit` n'est pas installé. Suggestion : `cargo install cargo-audit` et l'ajouter au CI.

### 5.3 Champ description manquant

`[workspace.package]` ne contient pas de `description`, et aucun sous-crate ne définit de description.

---

## 6. Problèmes de qualité du code

### 6.1 unwrap/expect dans le code de production

| Fichier | Ligne | Appel | Risque |
|------|------|------|------|
| `ecat-client/src/lib.rs` | 28 | `.expect("StaticResolver poisoned")` | Faible — raisonnable |
| `ecat/src/signal.rs` | 8 | `.expect("failed to install SIGTERM handler")` | Moyen — panic au démarrage |
| `ecat-protos/build.rs` | 5 | `.unwrap()` | Faible — script de build |

### 6.2 extract_version d'ecat-versioning

La fonction `extract_version` (ligne 68) extrait le numéro de version de l'en-tête Accept, mais n'est pas appelée par `build_header_router()`.

### 6.3 Gestion des erreurs d'ecat-data-questdb

```rust
// 第 30 行: 网络响应体读取使用 unwrap_or_default
Err(RdbmsError::Database(resp.text().await.unwrap_or_default()))
```

L'échec de `resp.text()` avale silencieusement le message d'erreur. Suggestion : remplacer par `unwrap_or_else(|e| format!("questdb parse: {e}"))`.

---

## 7. Évaluation de l'architecture

### Points forts

- Séparation claire des responsabilités sur 48 crates
- Version du workspace uniformisée `version.workspace = true`
- Dépendances épurées, sans gros framework
- Aucun TODO/FIXME/HACK

### À améliorer

| Problème | Priorité |
|------|--------|
| 50 % de crates sans tests | Élevée |
| Confusion yaml_serde vs serde_yaml | Moyenne |
| cargo-audit manquant | Moyenne |
| Code mort ecat-versioning | Faible |
| Pas de doc-tests | Faible |

---

## 8. Vue d'ensemble de la sécurité

| Élément de vérification | Résultat |
|--------|------|
| Clés codées en dur | Aucune trouvée |
| Fuite de fichiers .env | Aucune trouvée |
| unwrap dangereux (code de production) | 2 (signal.rs, client.rs) |
| Scan CVE | Non exécuté (cargo-audit à installer) |

---

## 9. Plan d'action

### P0 — Corrections immédiates
1. Nettoyer les 3 warnings du compilateur d'ecat-versioning
2. Corriger le clippy d'ecat-data-questdb
3. Corriger le derivable_impls d'ecat-client

### P1 — Court terme
4. Installer `cargo-audit` pour scanner les vulnérabilités des dépendances
5. Confirmer le choix `yaml_serde` vs `serde_yaml`
6. Compléter les doc-tests des crates centraux

### P2 — Moyen terme
7. Compléter les tests des crates transport/data/security
8. Ajouter le champ `description` à tous les crates
9. Intégrer ou supprimer `extract_version`

### P3 — Long terme
10. Mettre en place le CI : build → test → clippy → audit → coverage

---

*Rapport généré le 2026-08-01. Chaîne d'outils : cargo 1.92.0, rustc 1.92.0, clippy 1.92.0*
