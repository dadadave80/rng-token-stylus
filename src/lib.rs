#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
extern crate alloc;

use alloc::vec::Vec;
use openzeppelin_stylus::{
    token::erc20::{
        self,
        extensions::{Erc20Metadata, IErc20Metadata},
        Erc20, IErc20,
    },
    utils::introspection::erc165::IErc165,
};
use stylus_sdk::{
    alloy_primitives::{aliases::B32, Address, U256, U8},
    alloy_sol_types::sol,
    prelude::*,
    storage::{StorageAddress, StorageMap},
};

//*//////////////////////////////////////////////////////////////////////////
//                                 VRF SETUP
//////////////////////////////////////////////////////////////////////////*//

// Minimal interface for the Supra VRF Router Contract
// The `generateRequest` function is used to request randomness from Supra VRF
sol_interface! {
    interface ISupraRouterContract {
        function generateRequest(string memory function_sig, uint8 rng_count, uint256 num_confirmations, address client_wallet_address) external returns(uint256);
    }
}

sol! {
    // Thrown when a randomness request fails
    #[derive(Debug)]
    error RandomnessRequestFailed();
    // Thrown when a fulfillment is received from a non-Supra router
    #[derive(Debug)]
    error OnlySupraRouter();
}

// Custom events
sol! {
    event MintRequested(uint256 indexed nonce, address indexed to);
    event Minted(uint256 indexed nonce, address indexed to, uint256 amount);
}

#[derive(SolidityError, Debug)]
enum Error {
    InsufficientBalance(erc20::ERC20InsufficientBalance),
    InvalidSender(erc20::ERC20InvalidSender),
    InvalidReceiver(erc20::ERC20InvalidReceiver),
    InsufficientAllowance(erc20::ERC20InsufficientAllowance),
    InvalidSpender(erc20::ERC20InvalidSpender),
    InvalidApprover(erc20::ERC20InvalidApprover),
    // VRF Errors
    RandomnessRequestFailed(RandomnessRequestFailed),
    OnlySupraRouter(OnlySupraRouter),
}

impl From<erc20::Error> for Error {
    fn from(value: erc20::Error) -> Self {
        match value {
            erc20::Error::InsufficientBalance(e) => Error::InsufficientBalance(e),
            erc20::Error::InvalidSender(e) => Error::InvalidSender(e),
            erc20::Error::InvalidReceiver(e) => Error::InvalidReceiver(e),
            erc20::Error::InsufficientAllowance(e) => Error::InsufficientAllowance(e),
            erc20::Error::InvalidSpender(e) => Error::InvalidSpender(e),
            erc20::Error::InvalidApprover(e) => Error::InvalidApprover(e),
        }
    }
}

//*//////////////////////////////////////////////////////////////////////////
//                               LOTTERY TOKEN
//////////////////////////////////////////////////////////////////////////*//

#[entrypoint]
#[storage]
struct LotteryToken {
    erc20: Erc20,
    metadata: Erc20Metadata,
    subscription_manager: StorageAddress,
    supra_router: StorageAddress,
    mint_address: StorageMap<U256, StorageAddress>,
}

#[public]
#[implements(IErc20<Error = Error>, IErc20Metadata, IErc165)]
impl LotteryToken {
    #[constructor]
    pub fn constructor(
        &mut self,
        subscription_manager: Address,
        supra_router: Address,
    ) -> Result<(), Error> {
        self._init(subscription_manager, supra_router)
    }

    pub fn mint_to(&mut self, to: Address) -> Result<(), Error> {
        self._mint_to(to)
    }

    // Callback function from Supra VRF, called when the randomness is fulfilled
    // This is not meant to be called by users
    pub fn mint_random_amount(&mut self, nonce: U256, rng_list: Vec<U256>) -> Result<(), Error> {
        self._mint_random_amount(nonce, rng_list)
    }
}

impl LotteryToken {
    fn _init(&mut self, subscription_manager: Address, supra_router: Address) -> Result<(), Error> {
        self.metadata
            .constructor(String::from("Lottery Token"), String::from("LOTTO"));
        self.subscription_manager.set(subscription_manager);
        self.supra_router.set(supra_router);
        Ok(())
    }

    fn _mint_to(&mut self, to: Address) -> Result<(), Error> {
        let nonce = self._request_randomness()?;

        self.mint_address.setter(nonce).set(to);

        log(self.vm(), MintRequested { nonce, to });

        Ok(())
    }

    fn _mint_random_amount(&mut self, nonce: U256, rng_list: Vec<U256>) -> Result<(), Error> {
        // If the caller is not the Supra router, return an error
        if self.vm().msg_sender() != self.supra_router.get() {
            return Err(Error::OnlySupraRouter(OnlySupraRouter {}));
        }

        let receiver = self.mint_address.get(nonce);
        let random_num = rng_list[0];
        // Mint between 1 and 1,000 tokens
        let mint_range = U256::from(1000 * 10_u16.pow(18));
        let mint_amount = (random_num % mint_range) + U256::from(1);

        self.erc20._mint(receiver, mint_amount)?;

        log(
            self.vm(),
            Minted {
                nonce,
                to: receiver,
                amount: mint_amount,
            },
        );

        Ok(())
    }

    fn _request_randomness(&mut self) -> Result<U256, Error> {
        let subscription_manager = self.subscription_manager.get();
        let supra_router_address = self.supra_router.get();
        let router = ISupraRouterContract::from(supra_router_address);
        let request_result = router.generate_request(
            &mut *self,
            String::from("mintRandomAmount(uint256,uint256[])"),
            1,
            U256::from(1),
            subscription_manager,
        );

        match request_result {
            Ok(nonce) => Ok(nonce),
            Err(_) => Err(Error::RandomnessRequestFailed(RandomnessRequestFailed {})),
        }
    }
}

//*//////////////////////////////////////////////////////////////////////////
//                                ERC20 SETUP
//////////////////////////////////////////////////////////////////////////*//

#[public]
impl IErc20 for LotteryToken {
    type Error = Error;

    fn total_supply(&self) -> U256 {
        self.erc20.total_supply()
    }

    fn balance_of(&self, account: Address) -> U256 {
        self.erc20.balance_of(account)
    }

    fn transfer(&mut self, to: Address, value: U256) -> Result<bool, Self::Error> {
        Ok(self.erc20.transfer(to, value)?)
    }

    fn allowance(&self, owner: Address, spender: Address) -> U256 {
        self.erc20.allowance(owner, spender)
    }

    fn approve(&mut self, spender: Address, value: U256) -> Result<bool, Self::Error> {
        Ok(self.erc20.approve(spender, value)?)
    }

    fn transfer_from(
        &mut self,
        from: Address,
        to: Address,
        value: U256,
    ) -> Result<bool, Self::Error> {
        Ok(self.erc20.transfer_from(from, to, value)?)
    }
}

#[public]
impl IErc20Metadata for LotteryToken {
    fn name(&self) -> String {
        self.metadata.name()
    }

    fn symbol(&self) -> String {
        self.metadata.symbol()
    }

    fn decimals(&self) -> U8 {
        self.metadata.decimals()
    }
}

#[public]
impl IErc165 for LotteryToken {
    fn supports_interface(&self, interface_id: B32) -> bool {
        Erc20::supports_interface(&self.erc20, interface_id)
            || Erc20Metadata::supports_interface(&self.metadata, interface_id)
    }
}
