#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
extern crate alloc;

use alloc::vec::Vec;
use openzeppelin_stylus::{
    access::ownable::{self, IOwnable, Ownable},
    token::erc20::{
        self,
        extensions::{Erc20Metadata, IErc20Metadata},
        Erc20, IErc20,
    },
    utils::introspection::erc165::IErc165,
};
use stylus_sdk::{
    alloy_primitives::{aliases::B32, Address, U256, U8},
    prelude::*,
};

#[derive(SolidityError, Debug)]
enum Error {
    // Ownable Errors
    UnauthorizedAccount(ownable::OwnableUnauthorizedAccount),
    InvalidOwner(ownable::OwnableInvalidOwner),
    // ERC20 Errors
    InsufficientBalance(erc20::ERC20InsufficientBalance),
    InvalidSender(erc20::ERC20InvalidSender),
    InvalidReceiver(erc20::ERC20InvalidReceiver),
    InsufficientAllowance(erc20::ERC20InsufficientAllowance),
    InvalidSpender(erc20::ERC20InvalidSpender),
    InvalidApprover(erc20::ERC20InvalidApprover),
}

impl From<ownable::Error> for Error {
    fn from(value: ownable::Error) -> Self {
        match value {
            // If we get an UnauthorizedAccount error from the Ownable contract, map it to our UnauthorizedAccount error
            ownable::Error::UnauthorizedAccount(e) => Error::UnauthorizedAccount(e),
            // If we get an InvalidOwner error from the Ownable contract, map it to our InvalidOwner error
            ownable::Error::InvalidOwner(e) => Error::InvalidOwner(e),
        }
    }
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
struct RngToken {
    erc20: Erc20,
    metadata: Erc20Metadata,
    ownable: Ownable,
}

#[public]
#[implements(IErc20<Error = Error>, IErc20Metadata, IErc165, IOwnable<Error = Error>)]
impl RngToken {
    #[constructor]
    pub fn constructor(&mut self, initial_owner: Address) -> Result<(), Error> {
        self._init(initial_owner)
    }

    pub fn mint(&mut self, account: Address, value: U256) -> Result<(), Error> {
        self._mint(account, value)
    }
}

impl RngToken {
    fn _init(&mut self, initial_owner: Address) -> Result<(), Error> {
        self.ownable.constructor(initial_owner)?;
        self.metadata
            .constructor(String::from("Lucky Token"), String::from("LCK"));
        Ok(())
    }

    fn _mint(&mut self, account: Address, value: U256) -> Result<(), Error> {
        self.ownable.only_owner()?;
        self.erc20._mint(account, value)?;
        Ok(())
    }
}

//*//////////////////////////////////////////////////////////////////////////
//                                ERC20 SETUP
//////////////////////////////////////////////////////////////////////////*//

#[public]
impl IErc20 for RngToken {
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
impl IErc20Metadata for RngToken {
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
impl IOwnable for RngToken {
    type Error = Error;

    fn owner(&self) -> Address {
        self.ownable.owner()
    }

    fn transfer_ownership(&mut self, new_owner: Address) -> Result<(), self::Error> {
        Ok(self.ownable.transfer_ownership(new_owner)?)
    }

    fn renounce_ownership(&mut self) -> Result<(), self::Error> {
        Ok(self.ownable.renounce_ownership()?)
    }
}

#[public]
impl IErc165 for RngToken {
    fn supports_interface(&self, interface_id: B32) -> bool {
        Erc20::supports_interface(&self.erc20, interface_id)
            || Erc20Metadata::supports_interface(&self.metadata, interface_id)
    }
}
