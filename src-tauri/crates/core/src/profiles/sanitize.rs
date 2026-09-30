/// Longitud máxima del nombre de un perfil, en caracteres.
pub const MAX_PROFILE_NAME_LEN: usize = 64;

const INVALID_CHARS: [char; 10] = [':', '/', '\\', '?', '*', '"', '|', '%', '<', '>'];

const RESERVED_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Nombre de carpeta para un perfil: los caracteres `: / \ ? * " | % < >`, los
/// espacios y los de control pasan a `_`. Además se quitan los puntos finales y se evitan
/// los nombres reservados de Windows. Si queda vacío, `profile_<unix>`.
pub fn sanitize_directory_name(name: &str, now_unix: u64) -> String {
    let replaced: String = name
        .trim()
        .chars()
        .map(|c| {
            if INVALID_CHARS.contains(&c) || c.is_whitespace() || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = replaced.trim_end_matches('.');
    if trimmed.is_empty() {
        return format!("profile_{now_unix}");
    }
    let stem = trimmed.split('.').next().unwrap_or(trimmed);
    if RESERVED_NAMES.iter().any(|r| r.eq_ignore_ascii_case(stem)) {
        return format!("{trimmed}_");
    }
    trimmed.to_string()
}

/// Devuelve `base`, o `base_2`, `base_3`... hasta que `taken` diga que está libre.
pub fn resolve_unique(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_string();
    }
    (2u32..)
        .map(|n| format!("{base}_{n}"))
        .find(|candidate| !taken(candidate))
        .expect("an unbounded range always yields a free name")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_invalid_characters() {
        let cases = [
            ("Main", "Main"),
            ("My Smurf", "My_Smurf"),
            ("a:b/c\\d?e*f\"g|h%i<j>k", "a_b_c_d_e_f_g_h_i_j_k"),
            ("  padded  ", "padded"),
            ("tab\there", "tab_here"),
            ("dots...", "dots"),
            ("Ñandú #EUW", "Ñandú_#EUW"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                sanitize_directory_name(input, 0),
                expected,
                "input {input:?}"
            );
        }
    }

    #[test]
    fn empty_uses_timestamp() {
        assert_eq!(
            sanitize_directory_name("", 1700000000),
            "profile_1700000000"
        );
        assert_eq!(sanitize_directory_name("...", 5), "profile_5");
    }

    #[test]
    fn avoids_reserved_names() {
        assert_eq!(sanitize_directory_name("con", 0), "con_");
        assert_eq!(sanitize_directory_name("LPT1.txt", 0), "LPT1.txt_");
        assert_eq!(sanitize_directory_name("console", 0), "console");
    }

    #[test]
    fn resolves_collisions() {
        let taken = ["Main", "Main_2"];
        assert_eq!(resolve_unique("Main", |c| taken.contains(&c)), "Main_3");
        assert_eq!(resolve_unique("Alt", |c| taken.contains(&c)), "Alt");
    }
}
