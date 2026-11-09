# 1. Clean architecture en workspace Cargo

Les couches sont des crates séparés (`domain`, `application`, `infrastructure`, `ui`, `desktop`) plutôt que des modules d'un seul crate. Un module peut importer n'importe quel autre, alors qu'un crate ne voit que ce que son `Cargo.toml` déclare : le sens des dépendances est donc vérifié à la compilation. `domain` n'a aucune dépendance et `ui` ne connaît pas `infrastructure`.

Les ports sont des traits dynamiques (`Box<dyn SystemProbe>`), ce qui évite de propager des génériques jusque dans la vue. Le coût d'un appel virtuel toutes les 2 secondes est négligeable.
