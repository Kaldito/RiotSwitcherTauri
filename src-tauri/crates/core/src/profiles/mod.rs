//! Perfiles: modelo, base de datos `profiles_data.json`, saneado de nombres de carpeta y
//! fondos personalizados.

mod background;
mod model;
mod sanitize;
mod store;

pub use background::{ALLOWED_IMAGE_EXTENSIONS, import_background_image};
pub use model::{BackgroundRef, NewProfile, Profile, ProfilePatch};
pub use sanitize::{MAX_PROFILE_NAME_LEN, resolve_unique, sanitize_directory_name};
pub use store::{PROFILES_SCHEMA_VERSION, ProfileDb};
