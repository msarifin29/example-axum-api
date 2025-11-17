use std::{
    error::Error as fmt_error,
    fmt::{self, Display},
};

use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{Error, PasswordHasher, SaltString, rand_core::OsRng},
};

pub fn hash_password(pwd: String) -> Result<String, Error> {
    let number: &[u8] = pwd.as_bytes();
    let salt = SaltString::generate(OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(&number, &salt)?;

    Ok(password_hash.to_string())
}

pub fn parse_password(parse_pwd: &str) -> Result<PasswordHash<'_>, Error> {
    let parse_hash = PasswordHash::new(&parse_pwd)?;
    if parse_hash.hash.is_none() {
        return Err(Error::Password);
    }
    Ok(parse_hash)
}

#[derive(Debug)]
pub struct MsgError(pub String);

impl Display for MsgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt_error for MsgError {
    fn source(&self) -> Option<&(dyn fmt_error + 'static)> {
        None
    }
}

pub fn passwords_match(pwd: &str, new_pwd: &str) -> Result<bool, MsgError> {
    let parse_pwd = parse_password(pwd)
        .map_err(|e| MsgError(format!("Failed to parse password hash: {}", e)))?;

    Ok(Argon2::default()
        .verify_password(new_pwd.as_bytes(), &parse_pwd)
        .is_ok())
}

#[cfg(test)]
mod tests_util_password {
    use crate::auth::util::{hash_password, parse_password, passwords_match};
    use argon2::{
        Argon2,
        password_hash::{PasswordHash, PasswordVerifier},
    };

    #[test]
    fn test_hashing_password() {
        let password = "12345".to_string();

        let password_hash = hash_password(password.clone()).unwrap();
        let parsed_hash = PasswordHash::new(&password_hash).unwrap();
        assert!(
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed_hash)
                .is_ok()
        );
    }

    #[test]
    fn test_parsing_password() {
        let password = "12345".to_string();

        let password_hash = hash_password(password.clone()).unwrap();
        let parsed_hash = parse_password(&password_hash).unwrap();
        assert!(
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed_hash)
                .is_ok()
        );
    }

    #[test]
    fn test_parsing_password_failed() {
        let password = "12345".to_string();
        let password2 = "password".to_string();

        let password_hash = hash_password(password.clone()).unwrap();
        let parsed_hash = parse_password(&password_hash).unwrap();
        assert!(
            Argon2::default()
                .verify_password(password2.as_bytes(), &parsed_hash)
                .is_err()
        );
    }

    #[test]
    fn test_match_password() {
        let pwd = "12345".to_string();
        let hash = hash_password(pwd).unwrap();
        let new_pwd = "12345".to_string();
        let result = passwords_match(&hash, &new_pwd).unwrap();
        assert_eq!(result, true);
    }

    #[test]
    fn test_match_password_different() {
        let pwd = "12345".to_string();
        let hash = hash_password(pwd).unwrap();
        let new_pwd = "1234".to_string();
        let result = passwords_match(&hash, &new_pwd).unwrap();
        assert_eq!(result, false);
    }
}
