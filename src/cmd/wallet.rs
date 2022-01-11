
pub trait AccountBehaviour {
    type Address;

    type PublicKey;
    type SecretKey;

    type Signature;

    fn sign(&self, msg: &[u8]) -> Self::Address;
}

pub trait WalletBehaviour {
    type Account: AccountBehaviour;

    type PublicKey;
    type SecretKey;

    // type Iter: std::iter::Iterator<Item = (String, <Self::Account as AccountBehaviour>::Address)>;

    fn generate_account(&self, id: &str) -> Self::Account;
    fn import_account(&self, id: &str, sk: <Self::Account as AccountBehaviour>::SecretKey);
    fn export_account(&self, id: &str) -> Option<Self::Account>;
    fn remove_account(&self, id: &str);
    // fn list_account<'a>(&self) -> Self::Iter;
}

