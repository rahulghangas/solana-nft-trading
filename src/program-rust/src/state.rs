use solana_program::{
    program_pack::{IsInitialized, Pack, Sealed},
    program_error::ProgramError,
    pubkey::Pubkey,
};

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};

pub const STATESIZE: usize = 41usize;
pub const LISTESCROWSTATE: usize = 105usize;
pub const BIDESCROWSTATE: usize = 72usize;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlatformState {
    pub is_initialized: bool,
    pub authority: Pubkey,
    pub platform_fee: u64
}

impl Sealed for PlatformState{}

impl IsInitialized for PlatformState{
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for PlatformState {
    const LEN: usize = STATESIZE;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, PlatformState::LEN];
        let (
            is_initialized,
            authority,
            platform_fee
        ) = array_refs![src, 1, 32, 8];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };
        Ok(PlatformState{
            is_initialized,
            authority: Pubkey::new_from_array(*authority),
            platform_fee: u64::from_be_bytes(*platform_fee)
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, PlatformState::LEN];
        let (
            is_initialized_dst,
            authority_dst,
            platform_fee_dst
        ) = mut_array_refs![dst, 1, 32, 8];

        let PlatformState {
            is_initialized,
            authority,
            platform_fee
        } = self;

        is_initialized_dst[0] = *is_initialized as u8;
        authority_dst.copy_from_slice(authority.as_ref());
        *platform_fee_dst = platform_fee.to_be_bytes();
    }
}


#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ListEscrowState {
    pub lister: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub success: bool,
    pub successful_buyer: Pubkey,
}

impl Sealed for ListEscrowState{}

impl Pack for ListEscrowState {
    const LEN: usize = LISTESCROWSTATE;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, ListEscrowState::LEN];
        let (
            lister,
            mint, 
            amount,
            success,
            successful_buyer
        ) = array_refs![src, 32, 32, 8, 1, 32];
        let success = match success {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };
        Ok(ListEscrowState{
            lister: Pubkey::new_from_array(*lister),
            mint: Pubkey::new_from_array(*mint),
            amount: u64::from_be_bytes(*amount),
            success: success,
            successful_buyer: Pubkey::new_from_array(*successful_buyer),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, ListEscrowState::LEN];
        let (
            lister_dst,
            mint_dst,
            amount_dst,
            success_dst,
            successful_buyer_dst
        ) = mut_array_refs![dst, 32, 32, 8, 1, 32];

        let ListEscrowState {
            lister,
            mint,
            amount,
            success,
            successful_buyer
        } = self;

        lister_dst.copy_from_slice(lister.as_ref());
        mint_dst.copy_from_slice(mint.as_ref());
        *amount_dst = amount.to_be_bytes();
        success_dst[0] = *success as u8;
        successful_buyer_dst.copy_from_slice(successful_buyer.as_ref());
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BidEscrowState {
    pub bidder: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
}

impl Sealed for BidEscrowState{}

impl Pack for BidEscrowState {
    const LEN: usize = BIDESCROWSTATE;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, BidEscrowState::LEN];
        let (
            bidder,
            mint, 
            amount,
        ) = array_refs![src, 32, 32, 8];
        Ok(BidEscrowState{
            bidder: Pubkey::new_from_array(*bidder),
            mint: Pubkey::new_from_array(*mint),
            amount: u64::from_be_bytes(*amount),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, BidEscrowState::LEN];
        let (
            bidder_dst,
            mint_dst,
            amount_dst,
        ) = mut_array_refs![dst, 32, 32, 8];

        let BidEscrowState {
            bidder,
            mint,
            amount,
        } = self;

        bidder_dst.copy_from_slice(bidder.as_ref());
        mint_dst.copy_from_slice(mint.as_ref());
        *amount_dst = amount.to_be_bytes();
    }
}
