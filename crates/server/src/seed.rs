use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use sqlx::PgPool;
use uuid::Uuid;

/// Demo accounts share one password; it is published in the README for reviewers.
pub const DEMO_PASSWORD: &str = "ajaia-demo";

pub const USERS: [(&str, &str); 3] = [
    ("alice@ajaia.test", "Alice"),
    ("bob@ajaia.test", "Bob"),
    ("carol@ajaia.test", "Carol"),
];

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("argon2 hash")
        .to_string()
}

/// Idempotent: existing accounts are left untouched.
pub async fn seed_users(pool: &PgPool) -> sqlx::Result<()> {
    for (email, name) in USERS {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
            .bind(email)
            .fetch_one(pool)
            .await?;
        if exists {
            continue;
        }
        sqlx::query("INSERT INTO users (id, email, name, password_hash) VALUES ($1, $2, $3, $4)")
            .bind(Uuid::new_v4())
            .bind(email)
            .bind(name)
            .bind(hash_password(DEMO_PASSWORD))
            .execute(pool)
            .await?;
    }
    Ok(())
}
