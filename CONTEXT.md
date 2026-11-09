# Vocabulaire du projet

**Processus** : programme en cours d'exécution, identifié par son PID et son heure de démarrage, car l'OS peut réattribuer un PID.

**Application** : un ou plusieurs processus présentés à l'utilisateur comme un seul logiciel. Discord est une application composée de plusieurs processus.
_À éviter_ : « app » pour désigner un processus isolé.

**Mesure** : valeur de consommation d'une ressource (CPU, RAM, GPU, disque, réseau).

**Mesure indisponible** : l'OS ou la source n'a pas fourni la valeur. Elle s'affiche « — », reste en bas du classement et n'est jamais assimilée à 0.

**Échantillon** : relevé complet de la machine à un instant donné, toutes les 2 secondes.

**Tableau de bord** : état présenté à l'utilisateur après un échantillon : applications, totaux et historique.

**Historique** : dernières valeurs globales de chaque composant, sur 5 minutes, en mémoire uniquement.

**Application protégée** : application qu'on ne peut pas terminer depuis l'interface : composant de l'OS, exécutable inconnu ou Resource Monitor lui-même.

**Terminer** : demander l'arrêt de tous les processus d'une application, après confirmation.

**Sonde** : adaptateur qui produit les échantillons pour un OS donné (port `SystemProbe`).
