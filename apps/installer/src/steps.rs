//! The ordered steps of the install wizard and their titles.

/// One screen of the wizard, in flow order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Language,
    Keyboard,
    InstallType,
    Disk,
    Encryption,
    Account,
    Summary,
    Install,
    Done,
}

impl Step {
    /// All steps in display order.
    pub const ALL: [Step; 9] = [
        Step::Language,
        Step::Keyboard,
        Step::InstallType,
        Step::Disk,
        Step::Encryption,
        Step::Account,
        Step::Summary,
        Step::Install,
        Step::Done,
    ];

    /// Large heading for the step.
    pub fn title(self) -> &'static str {
        match self {
            Step::Language => "Welcome",
            Step::Keyboard => "Keyboard layout",
            Step::InstallType => "Install DraftOS",
            Step::Disk => "Choose a disk",
            Step::Encryption => "Encryption",
            Step::Account => "Create your account",
            Step::Summary => "Ready to install",
            Step::Install => "Installing DraftOS",
            Step::Done => "All done",
        }
    }

    /// One-line description under the title.
    pub fn subtitle(self) -> &'static str {
        match self {
            Step::Language => "Choose your language to get started.",
            Step::Keyboard => "Pick the layout that matches your keyboard.",
            Step::InstallType => "How would you like to install?",
            Step::Disk => "Select where DraftOS will be installed.",
            Step::Encryption => "Optionally protect your data with full-disk encryption.",
            Step::Account => "This account will be the administrator.",
            Step::Summary => "Review your choices before installing.",
            Step::Install => "Sit back — this will take a few minutes.",
            Step::Done => "DraftOS is installed and ready.",
        }
    }
}
