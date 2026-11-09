# Resource Monitor

Application de bureau en Rust et [GPUI](https://www.gpui.rs/) pour voir **quelles applications consomment le plus de ressources** : RAM, CPU, GPU, disque et réseau. Elle répond à des questions comme « est-ce que Discord prend beaucoup de RAM ? ».

- **Tableau de bord** : utilisation globale de chaque composant, courbe des 5 dernières minutes et application la plus consommatrice.
- **Classement par application** : les processus d'un même logiciel sont additionnés, et la ligne se déplie pour les détailler.
- **Tri** par colonne et **recherche** par nom.
- **Fermeture d'une application**, avec confirmation. Les composants de l'OS sont protégés.

Maquette : [Claude Design](https://claude.ai/artifact/GJbnMcxCJMKdZfBHXHuV3j) (lien privé).

## Lancer l'application

```sh
cargo run -p resource-monitor          # interface
cargo run -p infrastructure --example top   # classement dans le terminal, sans interface
cargo test --workspace
```

Il faut Rust stable et, sur macOS, Xcode. GPUI compile ses shaders au lancement (`runtime_shaders`), donc le Metal Toolchain n'est pas requis.

## Architecture

Le projet applique la clean architecture sous forme de workspace Cargo. Comme chaque couche est un crate, **le compilateur interdit les dépendances dans le mauvais sens**.

```text
desktop ──> ui ─────────────┐
   │                        ├──> application ──> domain
   └──────> infrastructure ─┘
```

| Crate | Rôle | Dépend de |
|---|---|---|
| `domain` | Règles métier : regroupement par application, tri, filtre, protection, historique | rien |
| `application` | Cas d'usage (`Monitor`, `terminate_app`) et ports (`SystemProbe`, `ProcessTerminator`) | `domain` |
| `infrastructure` | Adaptateurs OS : `sysinfo`, `nettop`, `ioreg` | `application`, `domain` |
| `ui` | Fenêtre GPUI et mise en forme | `application`, `domain` |
| `resource-monitor` (`crates/desktop`) | Racine de composition : assemble le tout | `application`, `infrastructure`, `ui` |

Le vocabulaire est défini dans [CONTEXT.md](CONTEXT.md) et les décisions dans [docs/adr](docs/adr).

## État par plateforme

| Mesure | macOS | Windows |
|---|---|---|
| CPU, RAM, disque | Oui | Oui, via `sysinfo` (non testé sur une vraie machine) |
| Réseau par application | Oui, via `nettop` | Pas encore : affiché « — » |
| GPU par application | Oui, via `ioreg` | Pas encore : affiché « — » |

La CI compile et teste sur macOS et Windows.

## Limites connues

- **Processus d'autres utilisateurs** : sans droits root, macOS masque leur CPU, leur RAM et leur disque, qui s'affichent « — ». Le total de la tuile Mémoire vient de l'OS et reste donc juste, même sans ces lignes.
- **RAM d'une application** : sur macOS, c'est la somme des empreintes physiques, comme dans le Moniteur d'activité. La mémoire compressée y compte à sa taille d'origine et le swap est inclus : quand la machine manque de RAM, la somme des lignes peut dépasser la mémoire installée. Sous la valeur de la tuile Mémoire, l'app indique la part du système (pages verrouillées, surtout le noyau) et celle du compresseur, qui ne figurent dans aucune ligne. Sur Windows, c'est la somme des mémoires résidentes (RSS), où la mémoire partagée peut être comptée plusieurs fois.
- **Regroupement** : les services WebKit de Safari apparaissent séparément, car ils ne vivent pas dans `Safari.app`.
- **GPU sur macOS** : `accumulatedGPUTime` n'est pas une API documentée par Apple et peut changer. La mesure devient alors « — ».
- **Recherche** : saisie clavier simple, sans méthode de saisie (IME).

## Feuille de route

- [ ] Réseau et GPU par application sur Windows
- [ ] Historique par application
- [ ] Icônes des applications
- [ ] Paquet `.app` signé et installeur Windows
