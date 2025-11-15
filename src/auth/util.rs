use argon2::{
    Argon2, PasswordHash,
    password_hash::{Error, PasswordHasher, SaltString, rand_core::OsRng},
};

pub fn hash_password(pwd: String) -> Result<String, Error> {
    let number: &[u8] = pwd.as_bytes();
    let salt = SaltString::generate(OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(&number, &salt)?.to_string();

    Ok(password_hash)
}

pub fn parse_password(parse_pwd: &str) -> Result<PasswordHash<'_>, Error> {
    let parse_hash = PasswordHash::new(&parse_pwd)?;
    if parse_hash.hash.is_none() {
        return Err(Error::Password);
    }
    Ok(parse_hash)
}

#[cfg(test)]
mod tests_util_password {
    use crate::auth::util::{hash_password, parse_password};
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
}
