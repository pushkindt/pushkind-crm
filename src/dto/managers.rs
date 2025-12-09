//! DTOs used in manager administration pages.

use crate::domain::client::Client;
use crate::domain::manager::Manager;

/// Data required to render the managers index page.
#[derive(Debug)]
pub struct ManagersPageData {
    /// Managers with their assigned clients.
    pub managers: Vec<(Manager, Vec<Client>)>,
}

/// Data displayed inside the manager modal.
#[derive(Debug)]
pub struct ManagerModalData {
    pub manager: Manager,
    pub clients: Vec<Client>,
}
