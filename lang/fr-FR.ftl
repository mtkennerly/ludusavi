ludusavi = Ludusavi
language = Langue
game-name = Nom
total-games = Jeux
file-size = Taille
file-location = Emplacement
overall = Général
status = Statut
cli-unrecognized-games = Pas d'informations pour ces jeux :
cli-unable-to-request-confirmation = Demande de confirmation impossible.
    .winpty-workaround = Si vous utilisez un émulateur Bash (comme Git Bash), essayez de lancer winpty.
cli-backup-id-with-multiple-games = Impossible de spécifier l'ID de sauvegarde lors de la restauration de plusieurs jeux.
cli-invalid-backup-id = ID de sauvegarde invalide.
badge-failed = ÉCHEC
badge-duplicates = DOUBLONS
badge-duplicated = DOUBLON
badge-ignored = IGNORÉ
badge-redirected-from = DE : { $path }
badge-redirecting-to = VERS : { $path }
some-entries-failed = Certaines entrées n'ont pas pu être traitées, recherchez { badge-failed } dans la sortie pour plus de détails. Vérifiez si vous pouvez accéder à ces fichiers ou si leurs chemins sont très longs.
cli-game-line-item-redirected = Redirigé depuis : { $path }
cli-game-line-item-redirecting = Redirigé vers : { $path }
button-backup = Sauvegarder
button-preview = Aperçu
button-restore = Restaurer
button-nav-backup = MODE DE SAUVEGARDE
button-nav-restore = MODE DE RESTAURATION
button-nav-custom-games = JEUX PERSONNALISÉS
button-nav-other = AUTRE
button-add-game = Ajouter un jeu
button-continue = Continuer
button-cancel = Annuler
button-cancelling = Annulation...
button-okay = Ok
button-select-all = Sélectionner tout
button-deselect-all = Désélectionner tout
button-enable-all = Activer tout
button-disable-all = Désactiver tout
button-customize = Personnaliser
button-exit = Quitter
button-comment = Commentaire
button-lock = Verrouiller
button-unlock = Déverrouiller
# This opens a download page.
button-get-app = Obtenir { $app }
button-validate = Valider
button-override-manifest = Remplacer le manifeste
button-extend-manifest = Étendre le manifeste
button-sort = Trier
button-download = Télécharger
button-upload = Téléverser
button-ignore = Ignorer
no-roots-are-configured = Ajoutez quelques dossiers pour sauvegarder encore plus de données.
config-is-invalid = Erreur : Le fichier de configuration est invalide.
manifest-is-invalid = Erreur : Le manifeste est invalide.
manifest-cannot-be-updated = Erreur : Impossible de vérifier la mise à jour du manifeste. Votre connexion Internet est-elle interrompue ?
cannot-prepare-backup-target = Erreur : Impossible de préparer la cible de sauvegarde (création ou vidage du dossier). Si vous avez le dossier ouvert dans votre explorateur de fichiers, essayez de le fermer : { $path }
restoration-source-is-invalid = Erreur : La source de restauration est invalide (soit elle n'existe pas, soit ce n'est pas un répertoire). Veuillez vérifier l'emplacement : { $path }
registry-issue = Erreur : Certaines entrées du registre ont été ignorées.
unable-to-browse-file-system = Erreur : Impossible de naviguer dans votre système.
unable-to-open-directory = Erreur : Impossible d'ouvrir le répertoire :
unable-to-open-url = Erreur : Impossible d’ouvrir l'URL :
unable-to-configure-cloud = Impossible de configurer le cloud.
unable-to-synchronize-with-cloud = Impossible de synchroniser avec le cloud.
cloud-synchronize-conflict = Vos sauvegardes locales et dans le cloud sont en conflit. Effectuez un chargement vers le cloud ou un téléchargement pour résoudre ce problème.
command-unlaunched = La commande n'a pas été lancée : { $command }
command-terminated = Commande interrompue brusquement : { $command }
command-failed = Échec de la commande avec le code { $code }: { $command }
processed-games =
    { $total-games } { $total-games ->
        [one] jeu
       *[other] jeux
    }
processed-games-subset =
    { $processed-games } sur { $total-games } { $total-games ->
        [one] jeu
       *[other] jeux
    }
processed-size-subset = { $processed-size } sur { $total-size }
field-backup-target = Sauvegarder vers :
field-restore-source = Restaurer depuis :
field-custom-files = Chemins :
field-custom-registry = Registre :
field-sort = Trier :
field-redirect-source =
    .placeholder = Source (Localisation d'origine)
field-redirect-target =
    .placeholder = Destination (Nouvelle localisation)
field-roots = Dossiers :
field-backup-excluded-items = Exclusions de sauvegarde :
field-redirects = Redirections :
# This appears next to the number of full backups that you'd like to keep.
# A full backup includes all save files for a game.
field-retention-full = Plein :
# This appears next to the number of differential backups that you'd like to keep.
# A differential backup includes only the files that have changed since the last full backup.
field-retention-differential = Différentielle :
field-backup-format = Format :
field-backup-compression = Compression :
# The compression level determines how much compresison we perform.
field-backup-compression-level = Niveau :
label-manifest = Manifeste
# This shows the time when we checked for an update to the manifest.
label-checked = Vérifier
# This shows the time when we found an update to the manifest.
label-updated = Mis à jour
label-new = Nouveau
label-removed = Retiré
label-comment = Commentaire
label-unchanged = Inchangé
label-backup = Sauvegarde
label-scan = Analyse
label-filter = Filtre
label-unique = Unique
label-complete = Terminé
label-partial = Partiel
label-enabled = Activé
label-disabled = Désactivé
# https://en.wikipedia.org/wiki/Thread_(computing)
label-threads = Threads
label-cloud = Cloud
# A "remote" is what Rclone calls cloud systems like Google Drive.
label-remote = Distant
label-remote-name = Nom distant
label-folder = Dossier
# An executable file
label-executable = Exécutable
# Options given to a command line program
label-arguments = Arguments
label-url = URL
# https://en.wikipedia.org/wiki/Host_(network)
label-host = Hôte
# https://en.wikipedia.org/wiki/Port_(computer_networking)
label-port = Port
label-username = Nom d’utilisateur
label-password = Mot de passe
# This is a specific website or service that provides some cloud functionality.
# For example, Nextcloud and Owncloud are providers of WebDAV services.
label-provider = Fournisseur
label-custom = Personnalisé
label-none = Aucun
label-change-count = Modifications : { $total }
label-unscanned = Non scanné
# This refers to a local file on the computer
label-file = Fichier
label-game = Jeu
# Aliases are alternative titles for the same game.
label-alias = Alias
label-original-name = Nom d'origine
# Which manifest a game's data came from
label-source = Source
# This refers to the main Ludusavi manifest: https://github.com/mtkennerly/ludusavi-manifest
label-primary-manifest = Manifeste principal
# This refers to how we integrate a custom game with the manifest data.
label-integration = Intégration
# This is a folder name where a specific game is installed
label-installed-name = Nom de l'installation
store-ea = EA
store-epic = Epic Games
store-gog = GOG
store-gog-galaxy = GOG Galaxy
store-heroic = Heroic
store-legendary = Legendary
store-lutris = Lutris
store-microsoft = Microsoft
store-origin = Origin
store-prime = Prime Gaming
store-steam = Steam
store-uplay = Uplay
store-other-home = Dossier personnel
# This would be a folder acting as a virtual C: drive, created by Wine.
store-other-wine = Préfixe Wine
# This would be a folder with typical Windows system folders,
# like "Program Files (x86)" and "Users".
store-other-windows = Disque Windows
# This would be a folder with typical Linux system folders,
# like "home" and "opt".
store-other-linux = Disque Linux
# This would be a folder with typical Mac system folders,
# like "Applications" and "Users".
store-other-mac = Disque Mac
store-other = Autres
backup-format-simple = Simple
backup-format-zip = Zip
compression-none = Aucun
# "Deflate" is a proper noun: https://en.wikipedia.org/wiki/Deflate
compression-deflate = Deflate
compression-bzip2 = Bzip2
compression-zstd = Zstd
theme = Thème
theme-light = Clair
theme-dark = Sombre
redirect-bidirectional = Mode bidirectionnel
reverse-redirects-when-restoring = Inverser la séquence des redirections lors de la restauration
show-disabled-games = Afficher les jeux désactivés
show-unchanged-games = Afficher les jeux non modifiés
show-unscanned-games = Afficher les jeux non scannés
override-max-threads = Outrepasser les threads max
synchronize-automatically = Synchroniser automatiquement
prefer-alias-display = Afficher l'alias au lieu du nom d'origine
skip-unconstructive-backups = Ignorer la sauvegarde quand les données seront supprimées, mais pas ajoutées ou mises à jour
explanation-for-exclude-store-screenshots = Dans les sauvegardes, excluez les captures d'écran spécifiques à la boutique
explanation-for-exclude-cloud-games = Ne pas sauvegarder les jeux avec la prise en charge du cloud sur ces plateformes
consider-doing-a-preview = Si vous ne l'avez pas déjà fait, pensez d'abord à faire un aperçu afin qu'il n'y ait pas de surprises.
confirm-backup =
    Êtes-vous sûr de vouloir procéder à la sauvegarde ? { $path-action ->
        [merge] Les nouvelles données de sauvegarde seront fusionnées dans le dossier cible :
       *[create] Le dossier cible sera créé :
    }
confirm-restore =
    Êtes-vous sûr de vouloir procéder à la restauration ?
    Cela écrasera tous les fichiers actuels avec les sauvegardes ici :
confirm-cloud-upload =
    Voulez-vous remplacer vos fichiers cloud par vos fichiers locaux ?
    Vos fichiers cloud ({ $cloud-path }) deviendront une copie exacte de vos fichiers locaux ({ $local-path }).
    Les fichiers dans le cloud seront mis à jour ou supprimés si nécessaire.
confirm-cloud-download =
    Voulez-vous remplacer vos fichiers locaux par vos fichiers cloud ?
    Vos fichiers locaux ({ $local-path }) deviendront une copie exacte de vos fichiers cloud ({ $cloud-path }).
    Les fichiers locaux seront mis à jour ou supprimés si nécessaire.
confirm-add-missing-roots = Ajouter ces dossiers ?
no-missing-roots = Aucun dossier supplémentaire trouvé.
loading = Chargement...
preparing-backup-target = Préparation du répertoire de sauvegarde...
updating-manifest = Mise à jour du manifeste...
no-cloud-changes = Aucun changement à synchroniser
backups-are-valid = Vos sauvegardes sont valides.
backups-are-invalid =
    Les sauvegardes de ces jeux semblent être invalides.
    Voulez-vous créer de nouvelles sauvegardes complètes pour ces jeux ?
saves-found = Données de sauvegarde trouvée.
no-saves-found = Aucune donnée de sauvegarde trouvée.
# This is tacked on to form something like "Back up (no confirmation)",
# meaning we would perform an action without asking the user if they're sure.
suffix-no-confirmation = sans confirmation
# This is shown when a setting will only take effect after closing and reopening Ludusavi.
suffix-restart-required = Redémarrage nécessaire
prefix-error = Erreur : { $message }
prefix-warning = Attention : { $message }
cloud-app-unavailable = Les sauvegardes dans le cloud sont désactivées car { $app } n'est pas disponible.
cloud-not-configured = Les sauvegardes dans le cloud sont désactivées car aucun système cloud n'est configuré.
cloud-path-invalid = Les sauvegardes dans le cloud sont désactivées car le chemin de sauvegarde est invalide.
game-is-unrecognized = Ludusavi ne reconnaît pas ce jeu.
game-has-nothing-to-restore = Ce jeu n'a pas de sauvegarde à restaurer.
launch-game-after-error = Lancez le jeu quand même ?
game-did-not-launch = Échec du lancement du jeu.
backup-is-newer-than-current-data = The existing backup is newer than the current data.
backup-is-older-than-current-data = The existing backup is older than the current data.
back-up-specific-game =
    .confirm = Sauvegarder les données pour { $game } ?
    .failed = Échec de la sauvegarde des données pour { $game }
restore-specific-game =
    .confirm = Restaurer les données de sauvegarde pour { $game } ?
    .failed = Échec de la restauration des données de sauvegarde pour { $game }
new-version-check = Vérifier automatiquement les mises à jour de l'application
new-version-available = Une mise à jour de l'application est disponible : { $version }. Souhaitez-vous voir les notes de version ?
custom-game-will-override = Ce jeu personnalisé remplace une entrée du manifeste
custom-game-will-extend = Ce jeu personnalisé étend une entrée du manifeste
operation-will-only-include-listed-games = Cette opération ne traitera que les jeux qui sont actuellement répertoriés
