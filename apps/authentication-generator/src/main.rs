use secp256k1::rand::rngs::OsRng;
use secp256k1::Secp256k1;

fn main() {
    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);
    println!("Secret Key: {:?}", secret_key.display_secret().to_string());
    println!("Public Key: {:?}", public_key.to_string());
}
