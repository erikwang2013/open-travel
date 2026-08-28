<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Rapport de revue de code e-cat (deuxième passe)

**Date :** 2026-07-29  
**Branche :** main  
**Projet :** e-cat (workspace Rust, 17 crates)

---

## I. Résumé de la revue

Sur la base des corrections clippy et du complément de tests de la première passe, cette passe a mené une revue approfondie de la logique du code, en mettant l'accent sur la correction du comportement à l'exécution, la sécurité de la concurrence et la cohérence sémantique de l'API. 32 fichiers sources ont été examinés.

### Ligne de base de validation

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

---

## II. Bugs découverts et corrections

### Bug 1 : [Critique] Erreur de durée de vie du gardien de span TracingLayer

- **Fichier :** `ecat-middleware/src/tracing.rs:37`
- **Sévérité :** **élevée**
- **Impact :** aucune requête passant par TracingLayer n'est couverte par un span de tracing

**Analyse de la cause racine :**

```rust
// 修复前
fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let _guard = span.enter();  // guard 在 call() 返回时 drop
    let fut = self.inner.call(req);
    Box::pin(fut)               // future 在后续 poll 时才执行
}
```

Le gardien retourné par `span.enter()` ne maintient le span actif que dans le contexte synchrone courant. `call()` retourne un future pas encore pollé ; l'exécution asynchrone réelle se produit lors des polls ultérieurs — à ce moment, le gardien a déjà été drop et le span n'a aucun effet. Aucune requête passant par TracingLayer n'apparaît dans la sortie de tracing.

**Correctif :**

```rust
// 修复后
use tracing::Instrument;

fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let fut = self.inner.call(req);
    Box::pin(fut.instrument(span))  // span 附着在 future 生命周期上
}
```

`tracing::Instrument::instrument()` attache le span au future, garantissant que le span reste actif pendant toute la durée de vie des polls du future.

---

### Bug 2 : [Critique] Défaut d'implémentation de la fermeture LifecycleHook — on_stop jamais exécuté

- **Fichier :** `ecat/src/hook.rs:14-23`, `ecat/src/lib.rs:11-16`
- **Sévérité :** **élevée**
- **Impact :** les hooks enregistrés via `.on_stop()` ne font rien à l'arrêt

**Analyse de la cause racine :**

Dans la conception d'origine, les méthodes `on_start()` et `on_stop()` poussaient toutes deux les hooks dans le même Vec `lifecycle_hooks`. Lors de `run()`, tous les hooks appelaient successivement `on_start()`, et à l'arrêt, tous les hooks appelaient successivement `on_stop()`.

Le problème vient de l'impl blanket du trait `LifecycleHook` pour les fermetures `Fn() -> Fut` : **elle ne couvre que `on_start()` ; `on_stop()` utilise l'implémentation par défaut du trait (no-op)**.

Cela signifie que lorsque l'utilisateur utilise la syntaxe de fermeture `.on_stop(|| async { ... })`, la fermeture est bien ajoutée à la liste des hooks, mais à l'arrêt, seul le `on_stop()` vide par défaut est exécuté — la logique de l'utilisateur ne s'exécute jamais.

**Correctif (deux parties) :**

1. **Séparation de start_hooks et stop_hooks** (`ecat/src/lib.rs`) :

```rust
// App 结构体 — 两个独立的 Vec
pub struct App {
    start_hooks: Vec<Box<dyn LifecycleHook>>,
    stop_hooks: Vec<Box<dyn LifecycleHook>>,
    // ...
}

// on_start() → start_hooks, on_stop() → stop_hooks
pub fn on_start(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.start_hooks.push(Box::new(hook));
    self
}
pub fn on_stop(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.stop_hooks.push(Box::new(hook));
    self
}
```

2. **Complétion de l'impl blanket des fermetures** (`ecat/src/hook.rs`) :

```rust
impl<F, Fut> LifecycleHook for F
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = Result<...>> + Send,
{
    async fn on_start(&self) -> ... { (self)().await }
    async fn on_stop(&self) -> ...  { (self)().await }  // 新增
}
```

Désormais, la fermeture implémente à la fois `on_start` et `on_stop` ; avec les Vec séparés, chaque hook n'est appelé qu'à la phase de cycle de vie correcte.

---

### Bug 3 : [Moyen] Priorité d'extraction du type de valeur Row de SqlxClient incorrecte

- **Fichier :** `ecat-data-sqlx/src/lib.rs:53-68`
- **Sévérité :** moyenne
- **Impact :** les valeurs entières et flottantes de la base de données sont extraites comme chaînes JSON plutôt que comme nombres

**Analyse de la cause racine :**

`try_get::<String>()` était tenté en premier. La plupart des pilotes de base de données réussissent `try_get::<String>()` sur des colonnes numériques (conversion implicite), ce qui fait que la valeur entière `42` est extraite comme `"42"` au lieu de `42`.

**Correctif :** réordonnancement de la séquence `try_get` en `i64 → f64 → String → Null`, en privilégiant la conservation des types numériques.

---

## III. Autres constats de la revue (non modifiés / limites connues)

| Catégorie | Fichier | Description | Suggestion |
|------|------|------|------|
| Fonctionnalité incomplète | `ecat-transport-http/src/lib.rs:30` | `axum::serve().await` bloque et ne retourne jamais, `stop()` est une opération vide | Implémenter un arrêt gracieux |
| Fonctionnalité incomplète | `ecat-transport-grpc/src/lib.rs:29` | Idem | Implémenter un arrêt gracieux |
| Fonctionnalité incomplète | `ecat-data-sqlx/src/lib.rs:79` | `transaction()` retourne une erreur non implémentée | Implémenter la prise en charge des transactions |
| Style de code | `ecat-middleware/src/logging.rs:42` | `elapsed.as_millis() as u64` troncature théorique u128→u64 | Aucun impact réel |
| Tests manquants | `ecat-middleware/` | 4 Tower Service sans tests unitaires | Nécessite des tests d'intégration |
| Tests manquants | `ecat-data/` | Définitions de traits pures | Acceptable pour l'instant |
| Blocage RwLock | `ecat-registry/src/memory.rs` | Le RwLock synchrone peut bloquer dans un contexte asynchrone | Envisager tokio::sync::RwLock |

---

## IV. Résultats des tests

```
cargo test → 60 passed, 0 failed

Répartition par crate :
  ecat                  4   (Builder/valeurs par défaut/hooks de cycle de vie)
  ecat-config           9   (parse env ×4 + config ×5)
  ecat-encoding        15   (JSON/Proto/CodecBox/codec_for/from_ct)
  ecat-errors           4   (mappage HTTP/conversion gRPC/metadata/Display)
  ecat-logging          1   (fumée init)
  ecat-metadata         9   (accès/From HeaderMap/From MetadataMap/itérateur)
  ecat-metrics          2   (singleton/text ne panique pas)
  ecat-registry         5   (enregistrement/découverte/désenregistrement/liste/filtrage)
  ecat-transport       11   (Context/Request/Response/trait Server)
  Les 8 autres crates   0   (traits purs/génération de code/tests d'intégration requis/impression pure)
```

---

## V. Liste des fichiers modifiés

| Fichier | Type de changement | Description du changement |
|------|----------|----------|
| `ecat/src/lib.rs` | Correction de bug | App séparé start_hooks/stop_hooks ; AppBuilder mis à jour en conséquence ; tests adaptés |
| `ecat/src/hook.rs` | Correction de bug | Complétion de l'impl blanket des fermetures avec on_stop() |
| `ecat-middleware/src/tracing.rs` | Correction de bug | Gardien de span → `fut.instrument(span)` |
| `ecat-data-sqlx/src/lib.rs` | Correction de bug | Ordre d'extraction des valeurs Row i64→f64→String→Null |

---

## VI. Résumé

Cette passe a découvert 2 bugs d'exécution à sévérité élevée et 1 problème de correction des données de sévérité moyenne :

1. **Span TracingLayer inopérant** — impacte l'observabilité de toutes les requêtes
2. **LifecycleHook on_stop non exécuté** — impacte la correction de toute la logique d'arrêt
3. **Perte du type numérique des Row** — impacte la correction de type des résultats de requêtes de base de données

Les trois problèmes sont corrigés ; après correction, les 60 tests passent, compilation sans erreur ni avertissement.

### Suggestions pour la suite

- Implémenter un arrêt gracieux pour les serveurs HTTP/gRPC
- Ajouter des tests d'intégration pour `ecat-middleware` (Service mock + validation des comportements span/délai d'attente/récupération)
- Ajouter des tests d'intégration pour `ecat-data-sqlx` (avec base de données SQLite en mémoire)
- Remplacer le RwLock synchrone de `ecat-registry/memory.rs` par `tokio::sync::RwLock`
