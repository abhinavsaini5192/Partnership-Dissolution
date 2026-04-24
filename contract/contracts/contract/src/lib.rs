#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Env, Symbol, Vec, Map
};

#[contract]
pub struct PartnershipContract;

#[contracttype]
#[derive(Clone)]
pub struct Partner {
    pub address: Address,
    pub share: u32, // must sum to 100
}

#[contracttype]
pub enum DataKey {
    Partners,
    Token,
    Initialized,
    Liquidated,
    Approvals, // Map<Address, bool>
}

#[contractimpl]
impl PartnershipContract {

    // 🔹 Initialize contract
    pub fn init(env: Env, partners: Vec<Partner>, token: Address) {
        if env.storage().instance().has(&DataKey::Initialized) {
            panic!("Already initialized");
        }

        // Validate total share = 100
        let mut total: u32 = 0;
        for p in partners.iter() {
            total += p.share;
        }
        if total != 100 {
            panic!("Shares must sum to 100");
        }

        env.storage().instance().set(&DataKey::Partners, &partners);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::Liquidated, &false);

        let approvals: Map<Address, bool> = Map::new(&env);
        env.storage().instance().set(&DataKey::Approvals, &approvals);
    }

    // 🔹 Deposit tokens into contract
    pub fn deposit(env: Env, from: Address, amount: i128) {
        from.require_auth();

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        client.transfer(&from, &env.current_contract_address(), &amount);
    }

    // 🔹 Approve liquidation (multi-sig)
    pub fn approve_liquidation(env: Env, partner: Address) {
        partner.require_auth();

        let partners: Vec<Partner> = env.storage().instance().get(&DataKey::Partners).unwrap();

        // Check if sender is valid partner
        let mut is_valid = false;
        for p in partners.iter() {
            if p.address == partner {
                is_valid = true;
            }
        }
        if !is_valid {
            panic!("Not a partner");
        }

        let mut approvals: Map<Address, bool> =
            env.storage().instance().get(&DataKey::Approvals).unwrap();

        approvals.set(partner, true);
        env.storage().instance().set(&DataKey::Approvals, &approvals);
    }

    // 🔹 Check if enough approvals (majority)
    fn has_majority(env: &Env) -> bool {
        let partners: Vec<Partner> = env.storage().instance().get(&DataKey::Partners).unwrap();
        let approvals: Map<Address, bool> = env.storage().instance().get(&DataKey::Approvals).unwrap();

        let mut count = 0;
        for p in partners.iter() {
            if approvals.get(p.address.clone()).unwrap_or(false) {
                count += 1;
            }
        }

        count > partners.len() / 2
    }

    // 🔹 Execute liquidation
    pub fn liquidate(env: Env) {
        let already: bool = env.storage().instance().get(&DataKey::Liquidated).unwrap();
        if already {
            panic!("Already liquidated");
        }

        if !Self::has_majority(&env) {
            panic!("Not enough approvals");
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        let contract_addr = env.current_contract_address();
        let total_balance = client.balance(&contract_addr);

        let partners: Vec<Partner> = env.storage().instance().get(&DataKey::Partners).unwrap();

        for p in partners.iter() {
            let payout = total_balance * (p.share as i128) / 100;
            client.transfer(&contract_addr, &p.address, &payout);
        }

        env.storage().instance().set(&DataKey::Liquidated, &true);
    }

    // 🔹 View functions
    pub fn is_liquidated(env: Env) -> bool {
        env.storage().instance().get(&DataKey::Liquidated).unwrap()
    }

    pub fn get_partners(env: Env) -> Vec<Partner> {
        env.storage().instance().get(&DataKey::Partners).unwrap()
    }
}