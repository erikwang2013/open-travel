<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Rapport de revue de code et de tests TDD e-cat

**Date :** 2026-07-29  
**Branche :** main  
**Projet :** e-cat (workspace Rust, 17 crates)

---

## I. Périmètre de la revue

Revue de tout le code source Rust des 17 crates du workspace (38 fichiers `.rs`).

| Crate | Description | Nombre de fichiers |
|-------|------|--------|
| `ecat-protos` | Définitions Protobuf et génération de code | 2 |
| `ecat-errors` | Types d'erreur unifiés | 2 |
| `ecat-metadata` | Abstraction des métadonnées de requête | 1 |
| `ecat-encoding` | Encodage/décodage JSON/Protobuf | 3 |
| `ecat-logging` | Initialisation des logs/Tracing | 1 |
| `ecat-config` | Chargement de configuration (fichier/variables d'environnement) | 3 |
| `ecat-data` | Abstraction par traits de la couche de données | 5 |
| `ecat-data-sqlx` | Implémentation RDBMS SQLx | 1 |
| `ecat-registry` | Découverte et enregistrement de services | 2 |
| `ecat-metrics` | Métriques Prometheus | 1 |
| `ecat-middleware` | Couche de middleware Tower | 4 |
| `ecat-transport` | Abstraction de la couche de transport | 4 |
| `ecat-transport-http` | Implémentation du transport HTTP/Axum | 1 |
| `ecat-transport-grpc` | Implémentation du transport gRPC/Tonic | 1 |
| `ecat` | Noyau du framework applicatif | 3 |
| `ecat-cli` | Outil CLI | 1 |
| `examples/helloworld` | Projet d'exemple | 1 |

---

## II. Problèmes découverts et corrections

### Problème 1 : [Clippy] `map_identity` — map d'identité inutile

- **Fichier :** `ecat-config/src/file.rs:30`
- **Sévérité :** faible
- **Problème :** `map(|(k, v)| (k, v))` n'effectue aucune transformation, c'est du code inutile
- **Correctif :** suppression de l'appel `.map()` superflu

### Problème 2 : [Clippy] `new_without_default` — Config sans implémentation Default

- **Fichier :** `ecat-config/src/lib.rs:27`
- **Sévérité :** faible
- **Problème :** `Config` a une méthode `new()` mais n'implémente pas le trait `Default`
- **Correctif :** remplacement de l'implémentation manuelle par `#[derive(Default)]`

### Problème 3 : [Clippy] `io_other_error` — ancien style de construction d'Error

- **Fichier :** `ecat-middleware/src/recovery.rs:42`
- **Sévérité :** faible
- **Problème :** `std::io::Error::new(std::io::ErrorKind::Other, ...)` a une alternative plus concise
- **Correctif :** remplacement par `std::io::Error::other("task panicked")`

### Problème 4 : [Clippy] `redundant_async_block` — bloc async redondant

- **Fichier :** `ecat-middleware/src/tracing.rs:38`
- **Sévérité :** faible
- **Problème :** dans `Box::pin(async move { fut.await })`, le bloc async est superflu
- **Correctif :** simplification en `Box::pin(fut)`

### Problème 5 : [Clippy] `redundant_closure` — fermeture redondante

- **Fichier :** `ecat-data-sqlx/src/lib.rs:63`
- **Sévérité :** faible
- **Problème :** la fermeture `.and_then(|f| serde_json::Number::from_f64(f))` peut être omise
- **Correctif :** utilisation directe de `.and_then(serde_json::Number::from_f64)`

### Problème 6 : [Clippy] `unwrap_or_default` — simplification possible

- **Fichier :** `ecat-transport-http/src/lib.rs:27`
- **Sévérité :** faible
- **Problème :** `unwrap_or_else(Router::new)` équivaut à `unwrap_or_default()`
- **Correctif :** remplacement par `unwrap_or_default()`

---

## III. Couverture des tests

### Avant corrections

| Crate | Nombre de tests |
|-------|--------|
| `ecat-errors` | 4 |
| `ecat-transport` | 11 |
| Les 15 autres crates | **0** |
| **Total** | **15** |

### Après corrections

| Crate | Nombre de tests | Ajoutés | Contenu des tests |
|-------|--------|------|----------|
| `ecat-encoding` | 15 | +15 | Allers-retours d'encodage/décodage JsonCodec, décodage invalide, content_type ; dispatch CodecBox ; chemins normal/erreur de codec_from_content_type ; variantes Encoding |
| `ecat-errors` | 4 | — | Mappage des statuts HTTP, conversion des statuts gRPC, accumulation des métadonnées, format Display |
| `ecat-metadata` | 9 | +9 | Accès clé-valeur, trace_id, From\<HeaderMap\> (saut des valeurs non UTF-8), From\<MetadataMap\> (ASCII et binaire ignorés), IntoIterator |
| `ecat-logging` | 1 | +1 | Test de fumée init |
| `ecat-config` | 4 | +4 | Nouvelle instance/valeurs par défaut, lecture typée, chargement depuis ConfigSource |
| `ecat-registry` | 5 | +5 | Enregistrement/découverte, désenregistrement/suppression, erreur si absent, liste des services, filtrage par nom |
| `ecat-metrics` | 2 | +2 | Registry singleton, metrics_text ne panique pas |
| `ecat` | 4 | +4 | Valeurs par défaut du Builder, nom/version personnalisés, enregistrement de server, hooks de cycle de vie |
| `ecat-transport` | 11 | — | Création de Context/Request/Response et valeurs par défaut, trait Server |
| **Total** | **55** | **+40** | |

### Crates sans tests unitaires

- `ecat-protos` — uniquement génération de code protobuf
- `ecat-data` — définitions de traits pures, aucune logique d'implémentation
- `ecat-data-sqlx` — nécessite une connexion à la base de données, relève des tests d'intégration
- `ecat-middleware` — implémentations Tower Service, nécessitent des tests d'intégration
- `ecat-transport-http` / `ecat-transport-grpc` — nécessitent une écoute réseau, relèvent des tests d'intégration
- `ecat-cli` — uniquement des sorties d'impression, aucune logique

---

## IV. Résultats de validation

```
cargo test   → 55 passed, 0 failed
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
```

---

## V. Liste des fichiers modifiés

| Fichier | Changement |
|------|------|
| `ecat-config/src/file.rs` | Suppression du map d'identité |
| `ecat-config/src/lib.rs` | `#[derive(Default)]` + 4 tests |
| `ecat-data-sqlx/src/lib.rs` | Simplification de la fermeture redondante |
| `ecat-middleware/src/recovery.rs` | Utilisation de `std::io::Error::other()` |
| `ecat-middleware/src/tracing.rs` | Suppression du bloc async redondant |
| `ecat-transport-http/src/lib.rs` | `unwrap_or_else` → `unwrap_or_default` |
| `ecat-metrics/src/lib.rs` | 2 tests |
| `ecat-registry/src/memory.rs` | 5 tests |
| `ecat/src/lib.rs` | 4 tests |
