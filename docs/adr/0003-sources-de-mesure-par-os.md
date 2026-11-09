# 3. Sources de mesure par OS

CPU, RAM et disque viennent de `sysinfo`, qui couvre macOS et Windows. Il ne fournit ni le réseau ni le GPU par processus.

Sur macOS, sans root :

- **RAM par processus** : `ri_phys_footprint` de `proc_pid_rusage`, car le RSS de `sysinfo` ignore les pages compressées et le swap : une VM de 7 Go pouvait s'afficher à 700 Mo. Le RSS reste le repli si la lecture échoue.
- **Mémoire hors applications** : pages verrouillées et pages du compresseur, lues avec `host_statistics64`. `sysinfo` les additionne dans la mémoire utilisée sans les exposer séparément.
- **Réseau** : compteurs cumulés de `nettop`, convertis en débit.
- **GPU** : `accumulatedGPUTime` des clients `IOAccelerator` dans l'IORegistry via `ioreg`, converti en pourcentage de temps. Cette propriété n'est pas documentée. `powermetrics` a été écarté car il exige root.

Une source en panne rend `None` et la mesure s'affiche « — » : l'application continue de fonctionner. Si une source répond sans citer un PID, celui-ci vaut 0, car il n'utilise pas la ressource.

Sur Windows, ces deux mesures sont pour l'instant indisponibles. Elles demanderont des compteurs PDH (`GPU Engine`) et ETW pour le réseau.

Le rendu GPUI utilise `runtime_shaders` pour ne pas exiger le Metal Toolchain à la compilation.
