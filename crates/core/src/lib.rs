use destiny_pkg::PackageManager;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref PACKAGE_MANAGER: PackageManager = PackageManager::new(
        std::env::args()
            .nth(1)
            .expect("Missing package directory argument"),
        destiny_pkg::GameVersion::Destiny2TheFinalShape,
        None,
    )
    .expect("Failed to initialize package manager");
}

pub fn package_manager() -> &'static PackageManager {
    &PACKAGE_MANAGER
}
