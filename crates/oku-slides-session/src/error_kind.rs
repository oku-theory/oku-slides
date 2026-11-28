//! The categories of session errors, which the interface and the tests tell apart.

/// The category of a [`SessionError`](crate::SessionError).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionErrorKind {
    /// A file that cannot be read or written.
    File,
    /// This PDF holds no Oku Slides sources.
    NotOkuSlides,
    /// Unreadable sources or deck.
    InvalidDocument,
    /// An operation of the transaction was refused.
    Op,
    /// Rendering missing or impossible: Typst failure, widgets unavailable, broken trusted pack.
    Render,
    /// Aucun fichier : il faut choisir où enregistrer.
    NoPath,
    /// No slide has the requested id.
    NoSlide,
    /// L'élément n'est pas un graphique (ou n'existe pas).
    NoChart,
    /// L'élément n'est pas un tableau (ou n'existe pas).
    NoTable,
    /// Le graphique n'a pas pu changer de type.
    Chart,
    /// The new chart type needs columns a data other views show lacks.
    ChartKindShared,
    /// Un pack intégré ne se supprime pas.
    BuiltinPack,
    /// Pas de dossier pour la bibliothèque de l'utilisateur.
    NoLibraryDir,
    /// Pack inconnu.
    UnknownPack,
    /// Version de pack introuvable.
    PackVersionMissing,
    /// A received pack the library lacks: the user adds it first.
    PackNotInstalled,
    /// A received version to add that the open deck does not hold.
    NotHeld,
    /// A received theme whose parent the library still lacks.
    ReceivedParentMissing,
    /// Pack du deck seul, que la bibliothèque n'enregistre pas.
    DeckOnly,
    /// Pack dont la version du deck ne peut devenir ma version courante.
    PackNotMakeCurrent,
    /// Le deck épingle déjà ma version courante du pack.
    PackAlreadyCurrent,
    /// Le thème du deck n'est pas plus ancien que ma version courante.
    ThemeNotOutdated,
    /// Espace de noms déjà pris par un autre pack de la bibliothèque.
    NamespaceTaken,
    /// Espace de noms `deck`, réservé au pack propre d'un deck.
    NamespaceReserved,
    /// Nouvelle version d'un pack sous un autre espace de noms que les siennes.
    NamespaceChanged,
    /// Nouvelle version d'un pack d'une autre sorte (widgets, layouts) que les siennes.
    PackKindChanged,
    /// Espace de noms qu'aucun pack ne porte (bibliothèque ou deck).
    UnknownNamespace,
    /// Packs qui se requièrent en boucle.
    RequiresCycle,
    /// L'import d'une présentation PowerPoint n'a pas abouti.
    Import,
    /// Un outil d'ajustement de l'import a refusé l'appel (voir `TuneErrorKind`).
    ImportTune,
    /// Requête impossible pendant l'édition d'un pack.
    EditingPack,
    /// A request a pack tab refuses: close it to finish, or open from the deck's tab.
    InPackTab,
    /// The library pack is being edited in another tab.
    PackOpenInTab,
    /// An undo or redo would change a pack a deck-pack tab edits.
    UndoInPackTab,
    /// A deck-pack tab's save no longer matches what its owner deck embeds.
    DeckPackMismatch,
    /// A refused undo or redo could not be taken back.
    UndoCompensationFailed,
    /// The library theme is being edited in another tab.
    ThemeOpenInTab,
    /// The offered pack version no longer applies to this deck.
    PackOfferExpired,
    /// The session given to a new pack tab was not fresh.
    PackTabNotFresh,
    /// A library pack written without the lease of its editor.
    LeaseMissing,
    /// Requête qui demande l'édition d'un pack.
    NotEditingPack,
    /// En mode pack, une op qui ajoute, retire, déplace une slide ou change son widget.
    PackModeSlides,
    /// En mode pack, une op sur une slide d'aperçu d'un pack de widgets.
    PackModePreview,
    /// A request that makes no sense for the session's kind of document.
    WrongDocument,
    /// A preview's ops include a knowledge op: previews show slide changes only.
    KnowledgeOpsInPreview,
    /// An edit or a move of a knowledge file whose text the user has not approved.
    KnowledgeChangeAwaiting,
    /// No knowledge file at that path.
    KnowledgeNotFound,
    /// The knowledge file awaits the user's approval: its text cannot be read.
    KnowledgeAwaiting,
    /// A malformed knowledge path.
    KnowledgePathInvalid,
    /// A malformed knowledge folder.
    KnowledgeDirInvalid,
    /// A knowledge search query that is empty or too long.
    KnowledgeQueryInvalid,
    /// The knowledge approvals could not be saved: nothing was approved.
    KnowledgeApprovalsWrite,
    /// An agent-authored knowledge file whose frontmatter cannot be stamped: nothing was applied.
    KnowledgeStamp,
    /// The system gave no random bytes for a knowledge stamp: nothing was applied.
    KnowledgeStampRandom,
    /// The knowledge approvals file exists but cannot be read or parsed: nothing was approved.
    KnowledgeApprovalsDamaged,
    /// No knowledge file has the id an approval names.
    KnowledgeIdNotFound,
    /// A knowledge file's text changed since the user was shown it: nothing was approved.
    KnowledgeChanged,
    /// A knowledge file over the read ceiling: it cannot be approved, only deleted.
    KnowledgeTooLargeToApprove,
    /// An approval that names a file twice or too many files.
    KnowledgeApprovalInvalid,
    /// A document import failed in its worker (time, memory, crash): nothing was imported.
    KnowledgeImportFailed,
    /// A document the extractor refuses (scan, legacy format, damaged…): nothing was imported.
    KnowledgeImportRefused,
    /// The attachment to import is no longer kept by the tab.
    AttachmentGone,
    /// The attachment to import is an image, not a document.
    NotADocument,
    /// An import target that is not a writable `/references/` file.
    ImportPathInvalid,
    /// An import target that an earlier import did not write.
    ImportTargetNotImport,
    /// An import target, or one of its parts, edited since the import wrote it.
    ImportTargetEdited,
    /// Another document import is running in the process: retry once it ends.
    KnowledgeImportBusy,
    /// The tab's document changed while a document was being extracted.
    ImportTabChanged,
    /// Réglage de l'hôte inconnu.
    HostKey,
    /// La réponse n'a pas pu être écrite en JSON.
    Response,
    /// An attachment refused, image or document ([`crate::SessionError::paste_reason_key`] says why).
    Paste,
    /// Une op cite un asset ni dans le deck ni parmi les images collées.
    UnknownAsset,
    /// An addition would push the deck's sources past `[sources] max_sources_mib`.
    SourcesOverBudget,
    /// A growing op would push the replica past 90 % of `[sources] max_replica_mib`.
    ReplicaFull,
    /// A column type change or a view conversion waits for, or outlived, a read of its data's base.
    BaseStale,
    /// A column insert or remove meets a view of its data that cannot follow it.
    ViewUnfollowed,
    /// A view bound to a query data whose result is pending or failed.
    QueryNoResult,
    /// A query data pasted into another deck has no ready result to copy.
    QueryPasteNoResult,
    /// A query data read while its result is pending or failed.
    QueryReadNoResult,
    /// A query preview reads a query data whose result is still pending.
    QueryInputPending,
    /// Arguments d'une surface de l'agent invalides.
    AgentForm,
    /// Adresse de l'agent : ni la slide ni un de ses éléments.
    UnknownOwner,
    /// Adresse de l'agent : pas un champ texte de l'objet.
    UnknownField,
    /// Adresse de l'agent : l'objet n'a pas de texte par défaut, il faut nommer son champ.
    FieldNeeded,
    /// Onglet inconnu (fermé entre-temps).
    UnknownTab,
    /// Fenêtre inconnue (fermée entre-temps).
    UnknownWindow,
    /// A request the detached agent window may not send.
    AgentWindowForbidden,
    /// Hôte à un seul document : pas d'autre onglet.
    SingleDocument,
    /// Fermeture refusée : travail non enregistré.
    Unsaved,
    /// Fusion refusée : ce PDF est une autre présentation.
    MergeUnrelated,
    /// Requête qui demande l'historique ouvert.
    HistoryNotOpen,
    /// Modifier, enregistrer ou fusionner pendant que l'historique est ouvert.
    InHistory,
    /// Un fichier existe déjà là où déplacer ou copier le deck.
    TargetExists,
    /// Modifier l'aide, montrée en lecture seule.
    ReadOnly,
    /// This tab cannot present its deck ([`crate::PresentRefusal`] says why).
    CannotPresent,
    /// Requête réservée à l'aide.
    NotHelp,
    /// A tab without a resume folder has no working folder for its agent.
    NoWorkDir,
    /// Annuler un tour de l'agent qui n'est plus la dernière modification du deck.
    TurnNotOnTop,
    /// Thème inconnu.
    UnknownTheme,
    /// Version de thème introuvable.
    ThemeVersionMissing,
    /// Des thèmes s'étendent en boucle.
    ThemeCycle,
    /// Un thème figure deux fois parmi ceux qu'étend le brouillon.
    ThemeExtendsDuplicate,
    /// Un parent du brouillon ne peut aller plus loin dans ce sens.
    ThemeExtendsMove,
    /// Les thèmes épinglés ne sont pas exactement le thème et ses parents.
    ThemeClosure,
    /// Valeur de jeton de design refusée par le schéma.
    InvalidTokenValue,
    /// Jeton de design encore référencé par d'autres jetons.
    TokenInUse,
    /// Nom, taille ou nombre de fichiers de widgets refusé.
    InvalidWidget,
    /// Agent guide too long or holding a NUL character.
    InvalidGuide,
    /// Widget utilisé par un modèle du pack.
    WidgetInUse,
    /// Fichier de widget introuvable.
    UnknownWidget,
    /// Le fichier du widget ne compile pas.
    WidgetInvalid,
    /// Essai d'un paramètre que le widget ne déclare pas.
    TrialUndeclared,
    /// Essai d'un paramètre non réglable dans l'aperçu.
    TrialFixed,
    /// Valeur d'essai trop longue.
    TrialTooLong,
    /// Valeur d'essai refusée pour le type du paramètre.
    TrialValue,
    /// Valeur d'essai impossible à garder.
    TrialEncode,
    /// A request's JSON longer than `MAX_APPLY_BYTES`.
    RequestTooLarge,
}
