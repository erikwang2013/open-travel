# Rapport de revue de code e-cat — 2026-08-01 (4e passe · tout corrigé)

**Version du projet :** 2.1.0  
**État final :** 0 warnings, ~116 tests, clippy propre, fmt propre

**Nettoyage de la 5e passe :** suppression de 12 dépendances inutilisées (ecat-health/reqwest, ecat-circuit-breaker/tokio, ecat-bench/tracing, ecat-mq/serde+serde_json, ecat-events/async-trait, ecat-config-remote/tracing, ecat-testing/transport-http+axum, ecat-client/serde+serde_json)
**Périmètre de la revue :** les 18 crates

## État final

| Outil | Statut |
|------|------|
| `cargo build` | Réussi (0 warnings) |
| `cargo test` | 77 passed, 0 failed, 1 ignored |
| `cargo clippy` | Réussi (0 warnings) |
| `cargo fmt` | Réussi |

---

## Liste des corrections (toutes)

### Risque moyen

1. **[Corrigé]** `Mutex::lock().unwrap()` → `ecat-transport-http/lib.rs`, `ecat-transport-grpc/lib.rs`
2. **[Corrigé]** CLI `fs::write().unwrap()` → `ecat-cli/src/main.rs`

### Risque faible

3. **[Corrigé]** Doc-test ProtoCodec → `ecat-encoding/src/proto.rs`
4. **[Corrigé]** Crates sans tests unitaires → 3 nouveaux tests chacun pour transport-http/grpc
5. **[Corrigé]** `Transaction::commit()` opération vide → nouveau trait `TransactionInner`
6. **[Corrigé]** Correction du commentaire de `SecurityScanner::new()`
7. **[Corrigé]** Dépendance `opentelemetry` inutilisée → `ecat-logging` et Cargo.toml racine du workspace
8. **[Corrigé]** Format des doc-tests

### Optimisations

9. **[Corrigé]** Préallocation de `scan_parts` → `Vec::with_capacity`
10. **[Corrigé]** `serde_yaml` 0.9 déprécié → migration vers `yaml_serde` 0.10
11. **[Corrigé]** `Transaction::commit()` n'est plus une opération vide → vrai commit/rollback via `SqlxTransactionWrapper`

### Sans correctif (décisions de conception)

- **Dépendances supplémentaires du crate `ecat`** — pattern « meta crate » délibéré, fournit des dépendances transitives pratiques aux utilisateurs en aval
- **Le trait Codec de ProtoCodec retourne une erreur** — différence de types fondamentale entre serde et prost::Message, traitée par la séparation des API `encode_message()`/`decode_message()` et une documentation claire
- **`ecat-data` sans implémentation concrète** — conception d'interfaces par traits, l'implémentation se trouve dans `ecat-data-sqlx`

---

## Récapitulatif des fichiers modifiés

| Fichier | Changement |
|------|------|
| `ecat-transport-http/src/lib.rs` | Protection contre l'empoisonnement Mutex + 3 nouveaux tests |
| `ecat-transport-grpc/src/lib.rs` | Protection contre l'empoisonnement Mutex + 3 nouveaux tests |
| `ecat-cli/src/main.rs` | Gestion des erreurs unifiée |
| `ecat-security/src/lib.rs` | Commentaire corrigé + optimisation de préallocation |
| `ecat-logging/Cargo.toml` | Suppression de opentelemetry inutilisé |
| `ecat-encoding/src/proto.rs` | Doc-tests améliorés |
| `ecat-data/src/lib.rs` | Export de TransactionInner |
| `ecat-data/src/rdbms.rs` | Nouveau trait TransactionInner |
| `ecat-data-sqlx/src/lib.rs` | SqlxTransactionWrapper implémente TransactionInner |
| `ecat-config/Cargo.toml` | serde_yaml → yaml_serde |
| `ecat-config/src/file.rs` | serde_yaml → yaml_serde |
| `Cargo.toml` | Suppression de la dépendance workspace opentelemetry orpheline |
| `README.md` | Mise à jour du numéro de version, correction de la description d'observabilité, ajout du lien du plan d'écosystème |
| `docs/ecosystem-plan.md` | Nouveau document de plan d'écosystème (15 crates en trois phases) |
