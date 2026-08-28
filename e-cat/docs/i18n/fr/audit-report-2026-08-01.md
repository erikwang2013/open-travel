# Rapport d'audit du framework e-cat — 2026-08-01

**Date d'audit :** 2026-08-01
**Périmètre de l'audit :** les 18 sous-crates (workspace)
**Chaîne d'outils :** stable (rustfmt, clippy)
**Résultats des tests :** 66 tests tous réussis | 0 échec | 0 ignoré

---

## 1. Évaluation générale

| Dimension | Note | Description |
|------|------|------|
| Compilation | ✅ Réussie | `cargo check` sans erreur, seulement 1 warning |
| Lint | ✅ Réussi | `cargo clippy --all-features` zéro avertissement |
| Tests | ✅ 66/66 | Tous les tests réussis |
| Couverture des tests | ⚠️ Insuffisante | 7 crates sans aucun test |
| Complétude fonctionnelle | ⚠️ Trop de stubs | ProtoCodec, Transaction, CLI new non implémentés |
| Qualité du code | ⚠️ Moyenne | Structure claire, mais plusieurs problèmes de conception |

---

## 2. Problèmes de compilation et de configuration

### 2.1 [WARNING] Clé manifest inutilisée

- **Fichier :** `/Cargo.toml:25`
- **Problème :** `workspace.package.name = "e-cat"` — ce champ n'a pas de sens au niveau du workspace, il produit un warning à chaque compilation
- **Correctif :** supprimer la ligne, ou la remplacer par un commentaire indiquant le nom du projet

### 2.2 [INFO] Éditions Rust incohérentes

- **Workspace :** `edition = "2026"`
- **Sous-crates :** `ecat-security/Cargo.toml` et `ecat-config/Cargo.toml` utilisent `edition = "2021"`
- **Remarque :** le workspace déclare l'édition 2026 mais certains sous-crates la remplacent par 2021. La compilation passe, mais l'édition 2026 n'est pas une édition stable officiellement publiée par Rust. Si c'est délibéré, vérifiez que la configuration de la chaîne d'outils est correcte
- **Suggestion :** confirmer que la chaîne d'outils prend en charge l'édition 2026, ou uniformiser vers 2024/2021

---

## 3. Fonctionnalités manquantes / Implémentations stub

### 3.1 [Critique] ProtoCodec totalement inutilisable

- **Fichier :** `ecat-encoding/src/proto.rs:8-10`
- **Problème :** `encode()` et `decode()` retournent toujours une erreur, le codec protobuf est entièrement un stub
- **Impact :** tout appel utilisant l'encodage protobuf échoue à l'exécution
- **Suggestion :** implémenter la liaison du trait prost::Message, ou fournir un flag de feature `prost` pour activer la fonctionnalité réelle

### 3.2 [Moyen] Transactions ecat-data-sqlx non implémentées

- **Fichier :** `ecat-data-sqlx/src/lib.rs:89-93`
- **Problème :** la méthode `transaction()` retourne l'erreur codée en dur `"transactions not yet implemented"`
- **Suggestion :** implémenter `pool.begin()` et retourner la Transaction encapsulée

### 3.3 [Moyen] HttpServer.stop() et GrpcServer.stop() sont des opérations vides

- **Fichiers :**
  - `ecat-transport-http/src/lib.rs:34-36`
  - `ecat-transport-grpc/src/lib.rs:33-35`
- **Problème :** la méthode `stop()` n'a aucune logique d'arrêt du serveur. `axum::serve()` et `tonic::Server::serve()` n'ont pas de mécanisme de réception du signal d'arrêt
- **Impact :** après l'appel à `App.run()`, le serveur continue de tourner quand `wait_for_shutdown` se déclenche ; impossible de fermer gracieusement
- **Suggestion :** utiliser `axum::serve(listener, router).with_graceful_shutdown(shutdown_signal)` et `tonic::Server::serve_with_shutdown()`

### 3.4 [Moyen] La commande CLI `new` est une coquille vide

- **Fichier :** `ecat-cli/src/main.rs:61-67`
- **Problème :** la commande `new` se contente d'imprimer un message, elle ne crée pas réellement les fichiers du modèle de projet
- **Suggestion :** implémenter la logique de génération du modèle, ou la marquer comme TODO

### 3.5 [Faible] Aucune implémentation dans la couche ecat-data

- **Fichier :** `ecat-data/src/{cache,graph,rdbms,search,tsdb}.rs`
- **Problème :** toutes les interfaces d'accès aux données n'ont que des définitions de traits, sans aucune implémentation (sauf `ecat-data-sqlx` qui fournit une implémentation de RdbmsClient)
- **Suggestion :** préciser dans le README l'état d'implémentation de chaque trait

---

## 4. Couverture des tests insuffisante

### 4.1 [Moyen] Crates sans aucune couverture de test (7)

| Crate | Fichiers sources | Remarque |
|-------|--------|------|
| `ecat-data` | 5 fichiers sources | Définitions de traits pures, aucun test |
| `ecat-data-sqlx` | 1 fichier source | Implémentation SQLx, aucun test d'intégration base de données |
| `ecat-middleware` | 4 fichiers sources | Les layers Logging/Recovery/Timeout/Tracing n'ont aucun test |
| `ecat-protos` | 1 fichier source | Code protobuf généré, aucun test |
| `ecat-transport-grpc` | 1 fichier source | Serveur gRPC, aucun test |
| `ecat-transport-http` | 1 fichier source | Serveur HTTP, aucun test |
| `ecat-cli` | 1 fichier source | Point d'entrée CLI, aucun test |

**Suggestions :**
- `ecat-middleware` : écrire des tests unitaires pour chaque layer avec `tower-test`
- `ecat-transport-http` : écrire des tests d'intégration du serveur HTTP avec `axum::test`
- `ecat-data-sqlx` : écrire des tests d'intégration base de données avec `sqlx::SqlitePool` (in-memory)

---

## 5. Qualité du code et problèmes de conception

### 5.1 [Critique] SecurityLayer détecte les attaques mais ne les bloque pas

- **Fichier :** `ecat-security/src/lib.rs:100-125`
- **Problème :** `SecurityService::call()` analyse les données de la requête et enregistre des alertes, mais transmet toujours la requête au service interne. Même en cas de détection d'injection SQL et d'attaque XSS, la requête est traitée normalement
- **Correctif :** retourner `403 Forbidden` ou `400 Bad Request` en cas de détection d'attaque

```rust
// 当前：总是转发
let fut = self.inner.call(req);
Box::pin(fut)

// 应改为：检测到高危攻击时拒绝
if results.iter().any(|r| r.severity >= Severity::High) {
    // 返回 403 响应
}
```

### 5.2 [Moyen] App::run() ne collecte pas les JoinHandle

- **Fichier :** `ecat/src/lib.rs:33-40`
- **Problème :** les `JoinHandle` retournés par `tokio::spawn` sont jetés, impossible de détecter un panic de serveur ou d'attendre l'arrêt gracieux
- **Suggestion :** collecter les JoinHandle dans un Vec et attendre la fermeture de tous les serveurs à l'arrêt

### 5.3 [Moyen] Registration::Drop échoue silencieusement quand le runtime est jeté

- **Fichier :** `ecat-registry/src/lib.rs:46-56`
- **Problème :** `Drop` appelle `tokio::spawn()` — si le runtime tokio a déjà été jeté, la tâche est ignorée silencieusement
- **Suggestion :** utiliser `tokio::task::block_in_place` + `Handle::block_on` ou passer par une méthode `unregister` explicite

### 5.4 [Moyen] Mappage des types de lignes de requête ecat-data-sqlx peu fiable

- **Fichier :** `ecat-data-sqlx/src/lib.rs:55-78`
- **Problème :** les valeurs des colonnes de la base sont essayées dans l'ordre `i64 → f64 → String → Null` ; certains pilotes peuvent signaler les valeurs entières comme des types incompatibles et provoquer des conversions erronées (par ex. PostgreSQL retourne INTEGER en `i32` plutôt qu'en `i64`)
- **Suggestion :** utiliser `ValueRef` / `TypeInfo` de SQLx pour vérifier le type réel de la colonne en base avant de décider de la stratégie de conversion

### 5.5 [Faible] Le contexte Metadata manque de méthodes de définition

- **Fichier :** `ecat-transport/src/context.rs:18-20`
- **Problème :** `Context` encapsule `Metadata` dans un `RwLock` et n'expose que la méthode de lecture `trace_id()`, impossible de définir trace_id ou d'autres métadonnées
- **Suggestion :** ajouter des méthodes d'écriture comme `set_trace_id()` à `Context`

### 5.6 [Faible] Les YAML/JSON non-objets de FileSource d'ecat-config sont ignorés silencieusement

- **Fichier :** `ecat-config/src/file.rs:30`
- **Problème :** `unwrap_or_default()` mappe les YAML non-objets (comme les tableaux `[1,2,3]` ou les valeurs scalaires) vers un HashMap vide, l'utilisateur peut ne pas comprendre pourquoi la configuration n'est pas chargée
- **Suggestion :** retourner `ConfigError::Other("expected object")`

---

## 6. Problèmes de compatibilité multiplateforme

### 6.1 [Moyen] Pas de support Ctrl+C pour wait_for_shutdown sur Windows

- **Fichier :** `ecat/src/signal.rs:13-14`
- **Problème :** sur les plateformes non-Unix, `terminate` est défini comme `std::future::pending::<()>()`, qui ne se résout jamais. Sur Windows, Ctrl+C est converti en signal SIGINT mais il n'est pas certain que `tokio::signal::ctrl_c()` fonctionne sur Windows
- **Suggestion :** utiliser aussi `tokio::signal::ctrl_c()` sur Windows (la documentation tokio dit qu'il prend en charge Windows), ou utiliser la série `tokio::signal::windows::ctrl_*`

---

## 7. Architecture et suggestions d'optimisation

### 7.1 [Optimisation] query() d'ecat-data-sqlx clone les noms de colonnes à répétition

- **Fichier :** `ecat-data-sqlx/src/lib.rs:48-83`
- **Problème :** le vecteur columns est cloné pour chaque ligne de données. Pour une requête qui retourne 1000 lignes, columns est cloné 1000 fois
- **Suggestion :** encapsuler columns dans un `Arc<Vec<String>>`, partagé par référence entre toutes les lignes

### 7.2 [Optimisation] Clonages inutiles dans MemoryRegistry::discover()

- **Fichier :** `ecat-registry/src/memory.rs:44-52`
- **Problème :** `.cloned()` clone toutes les ServiceInfo correspondantes. Si discover est appelé à haute fréquence, cela produit beaucoup d'allocations mémoire
- **Suggestion :** si l'appelant n'a pas besoin de la propriété, envisager de retourner `Vec<&ServiceInfo>` ou d'encapsuler dans `Arc<ServiceInfo>`

### 7.3 [Architecture] Suggestion de structure de re-export

Dans le crate `ecat-transport`, les paramètres génériques `T` de `Request` et `Response` valent par défaut `()`, il faut généralement spécifier le type concret à l'usage. Suggestion d'ajouter des alias de types :
```rust
pub type HttpRequest = Request<hyper::Body>;
pub type JsonRequest<T> = Request<T>;
```

### 7.4 [Sécurité] Middleware de limitation de débit manquant

La couche middleware manque actuellement de limitation de débit (Rate Limiting). Suggestion d'ajouter `RateLimitLayer` pour prévenir les attaques DoS.

---

## 8. Statistiques des tests

```
Vue d'ensemble des tests:
  Total: 66 tests
  Réussis: 66
  Échecs: 0
  Ignorés: 0

Répartition par crate:
  ecat:              4 tests ✅
  ecat-config:       9 tests ✅
  ecat-data:         0 tests ⚠️
  ecat-data-sqlx:    0 tests ⚠️
  ecat-encoding:    15 tests ✅
  ecat-errors:       4 tests ✅
  ecat-logging:      1 test  ✅
  ecat-metadata:     9 tests ✅
  ecat-metrics:      2 tests ✅
  ecat-middleware:   0 tests ⚠️
  ecat-protos:       0 tests ⚠️
  ecat-registry:     5 tests ✅
  ecat-security:     6 tests ✅
  ecat-transport:   11 tests ✅
  ecat-transport-grpc: 0 tests ⚠️
  ecat-transport-http: 0 tests ⚠️
  ecat-cli:          0 tests ⚠️
```

---

## 9. Récapitulatif des priorités des problèmes

| # | Sévérité | Problème | Fichier |
|---|--------|------|------|
| 1 | 🔴 Critique | SecurityLayer détecte les attaques mais ne les bloque pas | `ecat-security/src/lib.rs` |
| 2 | 🔴 Critique | ProtoCodec totalement inutilisable | `ecat-encoding/src/proto.rs` |
| 3 | 🟠 Moyen | stop() de HttpServer/GrpcServer est une opération vide | `ecat-transport-http/src/lib.rs`, `ecat-transport-grpc/src/lib.rs` |
| 4 | 🟠 Moyen | 7 crates sans aucune couverture de test | voir tableau 4.1 |
| 5 | 🟠 Moyen | App::run() ne collecte pas les JoinHandle | `ecat/src/lib.rs` |
| 6 | 🟠 Moyen | Transaction non implémentée | `ecat-data-sqlx/src/lib.rs` |
| 7 | 🟠 Moyen | Registration::Drop inopérant à la fermeture de tokio | `ecat-registry/src/lib.rs` |
| 8 | 🟠 Moyen | Mappage des types de colonnes d'ecat-data-sqlx peu fiable | `ecat-data-sqlx/src/lib.rs` |
| 9 | 🟠 Moyen | La commande CLI new est une coquille vide | `ecat-cli/src/main.rs` |
| 10 | 🟡 Faible | Warning de clé manifest inutilisée | `/Cargo.toml` |
| 11 | 🟡 Faible | Éditions incohérentes (2026 vs 2021) | `/Cargo.toml`, `ecat-security/Cargo.toml`, `ecat-config/Cargo.toml` |
| 12 | 🟡 Faible | Valeurs non-objets ignorées silencieusement par FileSource | `ecat-config/src/file.rs` |
| 13 | 🟡 Faible | Context manque de méthode set_trace_id | `ecat-transport/src/context.rs` |
| 14 | 🟡 Faible | Clonages inutiles dans discover() | `ecat-registry/src/memory.rs` |
| 15 | 🟡 Faible | Clonages répétés des columns dans query() | `ecat-data-sqlx/src/lib.rs` |
| 16 | 🟡 Faible | Middleware de limitation de débit manquant | — |

---

## 10. Résumé

La structure du framework est bien conçue, avec des couches claires ; la qualité de la compilation et du lint est bonne. Les principaux risques sont concentrés sur :
1. **SecurityLayer est un tigre de papier** — détecte mais ne bloque pas, c'est le problème le plus urgent à corriger
2. **ProtoCodec inutilisable** — si la prise en charge de protobuf est revendiquée, il faut l'implémenter
3. **L'arrêt gracieux des serveurs ne fonctionne pas** — impacte les déploiements en production
4. **Beaucoup de stubs et zéro couverture de test** — la maturité globale est encore à un stade précoce

Suggestion de corriger progressivement les problèmes ci-dessus dans l'ordre de priorité (critique → moyen → faible).

---

## 11. Journal des corrections (2026-08-01)

Tous les problèmes suivants ont été corrigés dans ce commit :

| # | Problème | Correctif | Statut |
|---|------|----------|------|
| 1 | SecurityLayer ne bloque pas | Type d'erreur `SecurityError` + blocage des attaques à haut risque avec `matches!` | ✅ Corrigé |
| 2 | ProtoCodec inutilisable | Ajout du flag de feature `prost-codec` + API `encode_message`/`decode_message` | ✅ Corrigé |
| 3 | stop() des serveurs vide | `watch::channel` + `with_graceful_shutdown` / `serve_with_shutdown` | ✅ Corrigé |
| 4 | 7 crates sans test | 4 nouveaux tests pour RateLimitLayer ; middleware a maintenant 4 tests | ✅ Partiellement corrigé |
| 5 | JoinHandle non collectés | Collecte `Vec<JoinHandle>` et await à l'arrêt | ✅ Corrigé |
| 6 | Transaction non implémentée | `pool.begin()` implémente la prise en charge des transactions | ✅ Corrigé |
| 7 | Registration::Drop | Détection sûre avec `tokio::runtime::Handle::try_current()` | ✅ Corrigé |
| 8 | Mappage des types de colonnes SQL | Ajout de chemins de prise en charge `bool` + `i32` | ✅ Corrigé |
| 9 | CLI new coquille vide | Génère réellement Cargo.toml, src/main.rs, proto/service.proto | ✅ Corrigé |
| 10 | Warning de clé manifest | Suppression de `workspace.package.name` | ✅ Corrigé |
| 11 | Éditions incohérentes | Uniformisation `edition.workspace = true` (2024) | ✅ Corrigé |
| 12 | Ignoré silencieusement par FileSource | `ok_or_else` retourne une erreur explicite | ✅ Corrigé |
| 13 | Context manque de méthodes | Ajout de `set_trace_id`, `set_meta`, `get_meta` | ✅ Corrigé |
| 14 | Clonages de discover() | `Arc<ServiceInfo>` réduit les clonages | ✅ Corrigé |
| 15 | Clonages des columns de query() | `Arc<Vec<String>>` partagé par référence | ✅ Corrigé |
| 16 | Limitation de débit manquante | Nouveau `RateLimitLayer` (token-bucket) + 4 tests | ✅ Corrigé |

### Nouveaux tests

- `ecat-middleware` : 4 tests RateLimitLayer (autorisation, blocage, clés séparées, construction)
- Nombre total de tests : 66 → 70

### Uniformisation des versions

- Workspace racine : `version = "1.0.3"`, `edition = "2024"`
- Tous les sous-crates : `version.workspace = true`, `edition.workspace = true`

### État de compilation final

- `cargo check --workspace` : ✅ réussi, zéro warning
- `cargo clippy --workspace --all-features` : ✅ réussi
- `cargo test --workspace` : ✅ 70/70 réussi
