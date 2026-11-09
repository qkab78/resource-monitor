# 2. Regroupement des processus par application

Le regroupement se fait à partir du chemin de l'exécutable : d'abord le premier bundle `.app` du chemin (macOS), sinon le nom de l'exécutable sans `.exe`, sinon le nom du processus.

Suivre l'arbre parent/enfant a été écarté : les helpers sont souvent relancés par `launchd` et perdent ce lien, et une app lancée depuis un terminal serait rattachée au terminal. Le chemin ne couvre pas tout (les services WebKit de Safari restent séparés). L'API privée de « responsible PID » de macOS pourrait combler ce manque plus tard.
