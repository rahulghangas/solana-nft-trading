use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    system_instruction,
    system_program,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_token;

use crate::{error::NFTError, instruction, instruction::NFTInstruction, state, state::{BidEscrowState, ListEscrowState, PlatformState}};

pub struct Processor;
impl Processor {
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
        let instruction = NFTInstruction::unpack(instruction_data)?;

        match instruction {
            NFTInstruction::Initialize(instruction::Initialize{authority, platform_fee}) => {
                msg!("Instruction: Initialize Platform");
                Self::process_init_platform(accounts, authority, platform_fee, program_id)
            }
            NFTInstruction::ChangeAuthority(instruction::ChangeAuthority{authority}) => {
                msg!("Instruction: Change Authority");
                Self::process_change_authority(accounts, authority, program_id)
            }
            NFTInstruction::ChangeFee(instruction::ChangeFee{platform_fee}) => {
                msg!("Instruction: Change Fee");
                Self::process_change_fee(accounts, platform_fee, program_id)
            }
            NFTInstruction::List(instruction::List{amount}) => {
                msg!("Instruction: List");
                Self::process_list(accounts, amount, program_id)
            }
            NFTInstruction::DeList(instruction::DeList{}) => {
                msg!("Instruction: Delist");
                Self::process_delist(accounts, program_id)
            }
            NFTInstruction::Bid(instruction::Bid{amount}) => {
                msg!("Instruction: Bid");
                Self::process_bid(accounts, amount, program_id)
            }
            NFTInstruction::WithdrawBid(instruction::WithdrawBid{}) => {
                msg!("Instruction: WithdrawBid");
                Self::process_withdraw_bid(accounts, program_id)
            }
            NFTInstruction::AcceptBid(instruction::AcceptBid{}) => {
                msg!("Instruction: AcceptBid");
                Self::process_accept_bid(accounts, program_id)
            }
            NFTInstruction::WithdrawNFTOnSuccess(instruction::WithdrawNFTOnSuccess{}) => {
                msg!("Instruction: WithdrawNFTOnSuccess");
                Self::process_withdraw_nft_on_success(accounts, program_id)
            }
            NFTInstruction::AcceptListing(instruction::AcceptListing{}) => {
                msg!("Instruction: AcceptBid");
                Self::process_accept_listing(accounts, program_id)
            }
            NFTInstruction::RefundUser(instruction::RefundUser{}) => {
                msg!("Instruction: RefundUser");
                Self::process_refund(accounts, program_id)
            }
        }
    }

    fn process_init_platform(
        accounts: &[AccountInfo],
        authority: Pubkey,
        platform_fee: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {  
        let account_info_iter = &mut accounts.iter();
        let initializer_info = next_account_info(account_info_iter)?;
        let state_account_info = next_account_info(account_info_iter)?;
        let vault_account_info = next_account_info(account_info_iter)?;
        let program_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let rent_account_info = next_account_info(account_info_iter)?;

        if !initializer_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        
        if !(program_info.key.eq(program_id)) {
            return Err(ProgramError::InvalidAccountData);
        }
        
        if !(system_program_info.key.eq(&system_program::id())) {
            return Err(ProgramError::InvalidAccountData);
        }

        let (state_account_pubkey, nonce1) = Pubkey::find_program_address(&[b"Platform", b"State"], program_id);
        if !(state_account_info.key.eq(&state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let (vault_account_pubkey, nonce2) = Pubkey::find_program_address(&[b"Platform", b"Vault"], program_id);
        if !(vault_account_info.key.eq(&vault_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let rent = &Rent::from_account_info(rent_account_info)?;
        let required_balance = rent.minimum_balance(state::STATESIZE);

        let create_state_account_ix = system_instruction::create_account(
            initializer_info.key, 
            &state_account_pubkey, 
            required_balance, 
            state::STATESIZE as u64, 
            program_id
        );
        invoke_signed(
            &create_state_account_ix,
            &[
                initializer_info.clone(),
                state_account_info.clone(),
                system_program_info.clone(),
                program_info.clone(),
            ],
            &[&[&b"Platform"[..], &b"State"[..], &[nonce1]]],
        )?;

        let required_balance = rent.minimum_balance(0);
        let create_vault_account_ix = system_instruction::create_account(
            initializer_info.key, 
            &vault_account_pubkey, 
            required_balance, 
            0, 
            program_id
        );
        invoke_signed(
            &create_vault_account_ix,
            &[
                initializer_info.clone(),
                vault_account_info.clone(),
                system_program_info.clone(),
                program_info.clone(),
            ],
            &[&[&b"Platform"[..], &b"Vault"[..], &[nonce2]]],
        )?;

        PlatformState::pack(
            PlatformState{
                is_initialized: true,
                authority: authority,
                platform_fee: platform_fee
            }, 
            &mut state_account_info.data.borrow_mut()
        )?;

        Ok(())
    }

    fn process_change_authority(
        accounts: &[AccountInfo],
        authority: Pubkey,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let initializer_info = next_account_info(account_info_iter)?;
        let state_account_info = next_account_info(account_info_iter)?;

        if !initializer_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        
        let (state_account_pubkey, _) = Pubkey::find_program_address(&[b"Platform", b"State"], program_id);
        let mut state_info = PlatformState::unpack_unchecked(&state_account_info.data.borrow())?;
        if !(state_account_info.key.eq(&state_account_pubkey)) {
            return Err(ProgramError::InvalidInstructionData);
        }
        if !state_info.is_initialized(){
            return Err(ProgramError::UninitializedAccount);
        }
        if !state_info.authority.eq(initializer_info.key) {
            return Err(NFTError::InvalidAuthority.into()); 
        }

        state_info.authority = authority;
        PlatformState::pack(state_info, &mut state_account_info.data.borrow_mut())?;

        Ok(())
    }

    fn process_change_fee(
        accounts: &[AccountInfo],
        platform_fee: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let initializer_info = next_account_info(account_info_iter)?;
        let state_account_info = next_account_info(account_info_iter)?;

        if !initializer_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let (state_account_pubkey, _) = Pubkey::find_program_address(&[b"Platform", b"State"], program_id);
        let mut state_info = PlatformState::unpack_unchecked(&state_account_info.data.borrow())?;
        if !(state_account_info.key.eq(&state_account_pubkey)) {
            return Err(ProgramError::InvalidInstructionData);
        }
        if !state_info.is_initialized(){
            return Err(ProgramError::UninitializedAccount);
        }
        if !state_info.authority.eq(initializer_info.key) {
            return Err(NFTError::InvalidAuthority.into()); 
        }

        state_info.platform_fee = platform_fee;
        PlatformState::pack(state_info, &mut state_account_info.data.borrow_mut())?;

        Ok(())
    }

    fn process_list(
        accounts: &[AccountInfo],
        amount: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let initializer_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let escrow_state_account_info = next_account_info(account_info_iter)?;
        let escrow_vault_account_info = next_account_info(account_info_iter)?;
        let program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let rent_account_info = next_account_info(account_info_iter)?;

        if !initializer_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let token_account_data = spl_token::state::Account::unpack_unchecked(&token_account_info.data.borrow())?;
        if !(token_account_data.owner.eq(&initializer_info.key)) {
            return Err(ProgramError::InvalidAccountData);
        }
        
        if !(mint_account_info.owner.eq(&spl_token::id())) {
            return Err(ProgramError::InvalidAccountData);
        }

        if !(token_account_data.mint.eq(&mint_account_info.key)) {
            return Err(ProgramError::InvalidAccountData);
        }
        
        if !(program_info.key.eq(program_id)) {
            return Err(ProgramError::InvalidAccountData);
        }
       
        if !(spl_token::id().eq(token_program_info.key)) {
            return Err(ProgramError::InvalidAccountData);
        }
            
        if !(system_program_info.key.eq(&system_program::id())) {
            return Err(ProgramError::InvalidAccountData);
        }

        let (escrow_state_account_pubkey, nonce1) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            initializer_info.key.as_ref(),
            b"List",
            b"State"
            ],
            program_id
        );
        if !(escrow_state_account_info.key.eq(&escrow_state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }
        let rent = &Rent::from_account_info(rent_account_info)?;
        let required_balance = rent.minimum_balance(state::LISTESCROWSTATE);
        let create_state_account_ix = system_instruction::create_account(
            initializer_info.key, 
            &escrow_state_account_pubkey, 
            required_balance, 
            state::LISTESCROWSTATE as u64, 
            program_id);
        invoke_signed(
            &create_state_account_ix,
            &[
                initializer_info.clone(),
                escrow_state_account_info.clone(),
                system_program_info.clone(),
                program_info.clone(),
            ],
            &[&[
                mint_account_info.key.as_ref(),
                initializer_info.key.as_ref(),
                &b"List"[..],
                &b"State"[..],
                &[nonce1]
            ]],
        )?;


        let (escrow_vault_account_pubkey, nonce2) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            initializer_info.key.as_ref(),
            b"List",
            b"Vault"
            ],
            program_id
        );
        if !(escrow_vault_account_info.key.eq(&escrow_vault_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }
        let required_balance = rent.minimum_balance(spl_token::state::Account::LEN);
        let create_vault_account_ix = system_instruction::create_account(initializer_info.key, &escrow_vault_account_pubkey, required_balance, spl_token::state::Account::LEN as u64, &spl_token::id());
        invoke_signed(
            &create_vault_account_ix,
            &[
                initializer_info.clone(),
                escrow_vault_account_info.clone(),
                system_program_info.clone(),
                token_program_info.clone(),
            ],
            &[&[
                mint_account_info.key.as_ref(),
                initializer_info.key.as_ref(),
                &b"List"[..],
                &b"Vault"[..],
                &[nonce2]
            ]],
        )?;

        let initialize_vault_account_ix = spl_token::instruction::initialize_account(
            &spl_token::id(), 
            &escrow_vault_account_pubkey, 
            mint_account_info.key, 
            &escrow_state_account_pubkey
        )?;
        invoke_signed(
            &initialize_vault_account_ix,
            &[
                token_program_info.clone(),
                escrow_vault_account_info.clone(),
                escrow_state_account_info.clone(),
                mint_account_info.clone(),
                program_info.clone(),
                rent_account_info.clone()
            ],
            &[&[
                mint_account_info.key.as_ref(),
                initializer_info.key.as_ref(),
                &b"List"[..],
                &b"Vault"[..],
                &[nonce2]
            ]],
        )?;

        let transfer_token_ix = spl_token::instruction::transfer_checked(
            &spl_token::id(),
            token_account_info.key, 
            mint_account_info.key, 
            &escrow_vault_account_pubkey, 
            initializer_info.key,
            &[
                initializer_info.key
            ], 
            1, 
            0
        )?;

        invoke(
            &transfer_token_ix,
            &[
                token_program_info.clone(),
                token_account_info.clone(),
                mint_account_info.clone(),
                escrow_vault_account_info.clone(),
                initializer_info.clone()
            ],
        )?;

        ListEscrowState::pack(
            ListEscrowState{
                lister: *initializer_info.key,
                amount: amount,
                mint: *mint_account_info.key,
                success: false,
                successful_buyer: Pubkey::new_from_array([0; 32])
            },
            &mut escrow_state_account_info.data.borrow_mut()
        )?;

        msg!("{{action: \"List\", lister \"{}\",, amount: {}, mint: {}}}", initializer_info.key, amount, mint_account_info.key);

        Ok(())
    }

    fn process_delist(
        accounts: &[AccountInfo],
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let signer_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let escrow_state_account_info = next_account_info(account_info_iter)?;
        let escrow_vault_account_info = next_account_info(account_info_iter)?;
        let program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        if !signer_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let token_account_data = spl_token::state::Account::unpack_unchecked(&token_account_info.data.borrow())?;
        if !(token_account_data.owner.eq(&signer_info.key)) {
            return Err(ProgramError::InvalidAccountData);
        }

        if !(mint_account_info.owner.eq(&spl_token::id())) {
            return Err(ProgramError::InvalidAccountData);
        }

        if !(program_info.key.eq(program_id)) {
            return Err(ProgramError::InvalidAccountData);
        }

        if !(spl_token::id().eq(token_program_info.key)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let (escrow_state_account_pubkey, nonce1) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            signer_info.key.as_ref(),
            b"List",
            b"State"
            ],
            program_id
        );
        if !(escrow_state_account_info.key.eq(&escrow_state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let (escrow_vault_account_pubkey, _) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            signer_info.key.as_ref(),
            b"List",
            b"Vault"
            ],
            program_id
        );
        if !(escrow_vault_account_info.key.eq(&escrow_vault_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let token_tansfer_ix = spl_token::instruction::transfer(
            &spl_token::id(), 
            &escrow_vault_account_pubkey, 
            token_account_info.key, 
            &escrow_state_account_pubkey, 
            &[&escrow_state_account_pubkey],
            1
        )?; 
        invoke_signed(
            &token_tansfer_ix,
            &[
                token_program_info.clone(),
                escrow_vault_account_info.clone(),
                token_account_info.clone(),
                escrow_state_account_info.clone()
            ],
            &[&[
                mint_account_info.key.as_ref(),
                signer_info.key.as_ref(),
                &b"List"[..],
                &b"State"[..],
                &[nonce1]
            ]],
        )?;

        let close_ix = spl_token::instruction::close_account(
            &spl_token::id(), 
            &escrow_vault_account_pubkey, 
            &escrow_state_account_pubkey, 
            &escrow_state_account_pubkey, 
            &[&escrow_state_account_pubkey]
        )?;

        invoke_signed(
            &close_ix,
            &[
                token_program_info.clone(),
                escrow_vault_account_info.clone(),
                signer_info.clone(),
                escrow_state_account_info.clone(),
            ],
            &[&[
                mint_account_info.key.as_ref(),
                signer_info.key.as_ref(),
                &b"List"[..],
                &b"State"[..],
                &[nonce1]
            ]],
        )?;

        let lamports = escrow_state_account_info.lamports();
        **escrow_state_account_info.try_borrow_mut_lamports()? = 0;
        **signer_info.try_borrow_mut_lamports()? += lamports;

        let list_state = ListEscrowState::unpack_unchecked(&escrow_state_account_info.data.borrow())?;
        if list_state.success {
            return Err(NFTError::ListingAlreadyFullfilled.into());
        }
        msg!("{{action: \"DeList\", lister \"{}\", amount: {}, mint: {}}}", signer_info.key, list_state.amount, mint_account_info.key);

        Ok(())
    }

    fn process_bid(
        accounts: &[AccountInfo],
        amount: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let initializer_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let escrow_state_account_info = next_account_info(account_info_iter)?;
        let escrow_vault_account_info = next_account_info(account_info_iter)?;
        let program_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let rent_account_info = next_account_info(account_info_iter)?;

        if !initializer_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
  
        if !(mint_account_info.owner.eq(&spl_token::id())) {
            return Err(ProgramError::InvalidAccountData);
        }

        if !(program_info.key.eq(program_id)) {
            return Err(ProgramError::InvalidAccountData);
        }
        
        
        if !(system_program_info.key.eq(&system_program::id())) {
            return Err(ProgramError::InvalidAccountData);
        }        

        let (escrow_state_account_pubkey, nonce1) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            initializer_info.key.as_ref(),
            b"Bid",
            b"State"
            ],
            program_id
        );
        if !(escrow_state_account_info.key.eq(&escrow_state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }
        let rent = &Rent::from_account_info(rent_account_info)?;
        let required_balance = rent.minimum_balance(state::BIDESCROWSTATE);
        let create_state_account_ix = system_instruction::create_account(
            initializer_info.key, 
            &escrow_state_account_pubkey, 
            required_balance, 
            state::BIDESCROWSTATE as u64, 
            program_id
        );
        invoke_signed(
            &create_state_account_ix,
            &[
                initializer_info.clone(),
                escrow_state_account_info.clone(),
                system_program_info.clone(),
                program_info.clone(),
            ],
            &[&[
                mint_account_info.key.as_ref(),
                initializer_info.key.as_ref(),
                &b"Bid"[..],
                &b"State"[..],
                &[nonce1]
            ]],
        )?;

        let (escrow_vault_account_pubkey, nonce2) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            initializer_info.key.as_ref(),
            b"Bid",
            b"Vault"
            ],
            program_id
        );
        if !(escrow_vault_account_info.key.eq(&escrow_vault_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }
        let required_balance = rent.minimum_balance(0);
        let create_vault_account_ix = system_instruction::create_account(
            initializer_info.key, 
            &escrow_vault_account_pubkey, 
            required_balance, 
            0, 
            program_id
        );
        invoke_signed(
            &create_vault_account_ix,
            &[
                initializer_info.clone(),
                escrow_vault_account_info.clone(),
                system_program_info.clone(),
                program_info.clone(),
            ],
            &[&[
                mint_account_info.key.as_ref(),
                initializer_info.key.as_ref(),
                &b"Bid"[..],
                &b"Vault"[..],
                &[nonce2]
            ]],
        )?;

        let transfer_lamports_ix = system_instruction::transfer(initializer_info.key, &escrow_vault_account_pubkey, amount);
        invoke(
            &transfer_lamports_ix,
            &[
                initializer_info.clone(),
                escrow_vault_account_info.clone(),
            ]
        )?;

        BidEscrowState::pack(
            BidEscrowState{
                bidder: *initializer_info.key,
                amount: amount,
                mint: *mint_account_info.key
            },
            &mut escrow_state_account_info.data.borrow_mut()
        )?;

        msg!("{{action: \"Bid\", bidder: \"{}\", amount: {}, mint: \"{}\"}}", initializer_info.key, amount, mint_account_info.key);

        Ok(())
    }

    fn process_withdraw_bid(
        accounts: &[AccountInfo],
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let signer_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let escrow_state_account_info = next_account_info(account_info_iter)?;
        let escrow_vault_account_info = next_account_info(account_info_iter)?;
        let program_info = next_account_info(account_info_iter)?;
        
        if !signer_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if !(mint_account_info.owner.eq(&spl_token::id())) {
            return Err(ProgramError::InvalidAccountData);
        }        

        if !(program_info.key.eq(program_id)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let (escrow_state_account_pubkey, _) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            signer_info.key.as_ref(),
            b"Bid",
            b"State"
            ],
            program_id
        );
        if !(escrow_state_account_info.key.eq(&escrow_state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let (escrow_vault_account_pubkey, _) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            signer_info.key.as_ref(),
            b"Bid",
            b"Vault"
            ],
            program_id
        );
        if !(escrow_vault_account_info.key.eq(&escrow_vault_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let lamports = escrow_state_account_info.lamports() + escrow_vault_account_info.lamports();
        **escrow_state_account_info.try_borrow_mut_lamports()? = 0;
        **escrow_vault_account_info.try_borrow_mut_lamports()? = 0;
        **signer_info.try_borrow_mut_lamports()? += lamports;

        let bid_state = BidEscrowState::unpack_unchecked(&escrow_state_account_info.data.borrow())?;
        msg!("{{action: \"WithdrawBid\", bidder: \"{}\", amount: {}, mint: {}}}", signer_info.key, bid_state.amount, mint_account_info.key);

        Ok(())
    }

    fn process_accept_bid(
        accounts: &[AccountInfo],
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let signer_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let bidder_account_info = next_account_info(account_info_iter)?;
        let state_account_info = next_account_info(account_info_iter)?;
        let vault_account_info = next_account_info(account_info_iter)?;
        let escrow_bid_state_account_info = next_account_info(account_info_iter)?;
        let escrow_bid_vault_account_info = next_account_info(account_info_iter)?;
        let escrow_list_state_account_info = next_account_info(account_info_iter)?;
        let escrow_list_vault_account_info = next_account_info(account_info_iter)?;

        if !signer_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if !(mint_account_info.owner.eq(&spl_token::id())) {
            return Err(ProgramError::InvalidAccountData);
        }

        let (state_account_pubkey, _) = Pubkey::find_program_address(&[b"Platform", b"State"], program_id);
        if !(state_account_info.key.eq(&state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }
        let (vault_account_pubkey, _) = Pubkey::find_program_address(&[b"Platform", b"Vault"], program_id);
        if !(vault_account_info.key.eq(&vault_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let (escrow_bid_state_account_pubkey, _) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            bidder_account_info.key.as_ref(),
            b"Bid",
            b"State"
            ],
            program_id
        );
        if !(escrow_bid_state_account_info.key.eq(&escrow_bid_state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }
        let (escrow_bid_vault_account_pubkey, _) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            bidder_account_info.key.as_ref(),
            b"Bid",
            b"Vault"
            ],
            program_id
        );
        if !(escrow_bid_vault_account_info.key.eq(&escrow_bid_vault_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }
        
        let (escrow_list_state_account_pubkey, _) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            signer_info.key.as_ref(),
            b"List",
            b"State"
            ],
            program_id
        );
        if !(escrow_list_state_account_info.key.eq(&escrow_list_state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }
        let (escrow_list_vault_account_pubkey, _) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            signer_info.key.as_ref(),
            b"List",
            b"Vault"
            ],
            program_id
        );
        if !(escrow_list_vault_account_info.key.eq(&escrow_list_vault_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let platform_state = PlatformState::unpack_unchecked(&state_account_info.data.borrow())?;

        let mut list_state = ListEscrowState::unpack_unchecked(&escrow_list_state_account_info.data.borrow())?;
        if !list_state.lister.eq(signer_info.key) {
            return Err(ProgramError::InvalidAccountData);
        }
        if list_state.success {
            return Err(NFTError::ListingAlreadyFullfilled.into());
        }

        let bid_state = BidEscrowState::unpack_unchecked(&escrow_bid_state_account_info.data.borrow())?;
        if !bid_state.bidder.eq(bidder_account_info.key) {
            return Err(ProgramError::InvalidAccountData);
        }

        list_state.amount = bid_state.amount;
        list_state.success = true;
        list_state.successful_buyer = *bidder_account_info.key;

        ListEscrowState::pack(
            list_state,
            &mut escrow_list_state_account_info.data.borrow_mut()
        )?;

        let fees_to_platform = (bid_state.amount * platform_state.platform_fee ) / 1000000000;
        let amount_after_fees = bid_state.amount - fees_to_platform;
        let lamports_to_bidder = escrow_bid_state_account_info.lamports() + escrow_bid_vault_account_info.lamports() - bid_state.amount;
        **escrow_bid_state_account_info.try_borrow_mut_lamports()? = 0;
        **escrow_bid_vault_account_info.try_borrow_mut_lamports()? = 0;
        **vault_account_info.try_borrow_mut_lamports()? += fees_to_platform;
        **signer_info.try_borrow_mut_lamports()? += amount_after_fees;
        **bidder_account_info.try_borrow_mut_lamports()? += lamports_to_bidder;

        let log = format!("{{action: \"AcceptBid\", bidder: \"{}\", lister: \"{}\", amount: {}, mint: \"{}\"}}", bidder_account_info.key.to_string(), signer_info.key.to_string(), bid_state.amount, list_state.mint.to_string());
        msg!(&log);

        Ok(())
    }

    fn process_withdraw_nft_on_success(
        accounts: &[AccountInfo],
        program_id: &Pubkey,
    ) -> ProgramResult{
        let account_info_iter = &mut accounts.iter();
        let signer_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let lister_account_info = next_account_info(account_info_iter)?;
        let escrow_list_state_account_info = next_account_info(account_info_iter)?;
        let escrow_list_vault_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        if !signer_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let token_account_data = spl_token::state::Account::unpack_unchecked(&token_account_info.data.borrow())?;
        if !(token_account_data.owner.eq(&signer_info.key)) {
            return Err(ProgramError::InvalidAccountData);
        }

        if !(mint_account_info.owner.eq(&spl_token::id())) {
            return Err(ProgramError::InvalidAccountData);
        }

        if !(token_account_data.mint.eq(&mint_account_info.key)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let (escrow_list_state_account_pubkey, nonce1) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            lister_account_info.key.as_ref(),
            b"List",
            b"State"
            ],
            program_id
        );
        if !(escrow_list_state_account_info.key.eq(&escrow_list_state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }
        let (escrow_list_vault_account_pubkey, _) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            lister_account_info.key.as_ref(),
            b"List",
            b"Vault"
            ],
            program_id
        );
        if !(escrow_list_vault_account_info.key.eq(&escrow_list_vault_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        if !(spl_token::id().eq(token_program_info.key)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let list_state = ListEscrowState::unpack_unchecked(&escrow_list_state_account_info.data.borrow())?;
        if !list_state.lister.eq(lister_account_info.key) {
            return Err(ProgramError::InvalidAccountData);
        }
        if !list_state.success {
            return Err(ProgramError::InvalidAccountData);
        }
        if !list_state.successful_buyer.eq(signer_info.key) {
            return Err(ProgramError::InvalidAccountData);
        }

        let nft_transfer_ix = spl_token::instruction::transfer_checked(
            &spl_token::id(),
            &escrow_list_vault_account_pubkey,
            mint_account_info.key,
            token_account_info.key,
            &escrow_list_state_account_pubkey,
            &[&escrow_list_state_account_pubkey],
            1,
            0
        )?;
        
        invoke_signed(
            &nft_transfer_ix,
            &[
                token_program_info.clone(),
                escrow_list_vault_account_info.clone(),
                mint_account_info.clone(),
                token_account_info.clone(),
                escrow_list_state_account_info.clone(),
            ],

            &[&[
                mint_account_info.key.as_ref(),
                lister_account_info.key.as_ref(),
                &b"List"[..],
                &b"State"[..],
                &[nonce1]
            ]]
        )?;

        let close_ix = spl_token::instruction::close_account(
            &spl_token::id(), 
            &escrow_list_vault_account_pubkey, 
            &escrow_list_state_account_pubkey, 
            &escrow_list_state_account_pubkey, 
            &[&escrow_list_state_account_pubkey]
        )?;

        invoke_signed(
            &close_ix,
            &[
                token_program_info.clone(),
                escrow_list_vault_account_info.clone(),
                signer_info.clone(),
                escrow_list_state_account_info.clone(),
            ],
            &[&[
                mint_account_info.key.as_ref(),
                lister_account_info.key.as_ref(),
                &b"List"[..],
                &b"State"[..],
                &[nonce1]
            ]],
        )?;

        let lamports = escrow_list_state_account_info.lamports();
        **escrow_list_state_account_info.try_borrow_mut_lamports()? = 0;
        **lister_account_info.try_borrow_mut_lamports()? += lamports;

        Ok(())
    }

    fn process_accept_listing(
        accounts: &[AccountInfo],
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let signer_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let lister_account_info = next_account_info(account_info_iter)?;
        let state_account_info = next_account_info(account_info_iter)?;
        let vault_account_info = next_account_info(account_info_iter)?;
        let escrow_list_state_account_info = next_account_info(account_info_iter)?;
        let escrow_list_vault_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        if !signer_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let token_account_data = spl_token::state::Account::unpack_unchecked(&token_account_info.data.borrow())?;
        if !(token_account_data.owner.eq(&signer_info.key)) {
            return Err(ProgramError::InvalidAccountData);
        }
        
        if !(mint_account_info.owner.eq(&spl_token::id())) {
            return Err(ProgramError::InvalidAccountData);
        }

        if !(token_account_data.mint.eq(&mint_account_info.key)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let (state_account_pubkey, _) = Pubkey::find_program_address(&[b"Platform", b"State"], program_id);
        if !(state_account_info.key.eq(&state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let (vault_account_pubkey, _) = Pubkey::find_program_address(&[b"Platform", b"Vault"], program_id);
        if !(vault_account_info.key.eq(&vault_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let (escrow_list_state_account_pubkey, nonce1) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            lister_account_info.key.as_ref(),
            b"List",
            b"State"
            ],
            program_id
        );
        if !(escrow_list_state_account_info.key.eq(&escrow_list_state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }
        let (escrow_list_vault_account_pubkey, _) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            lister_account_info.key.as_ref(),
            b"List",
            b"Vault"
            ],
            program_id
        );
        if !(escrow_list_vault_account_info.key.eq(&escrow_list_vault_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        if !(spl_token::id().eq(token_program_info.key)) {
            return Err(ProgramError::InvalidAccountData);
        }

        if !(system_program_info.key.eq(&system_program::id())) {
            return Err(ProgramError::InvalidAccountData);
        }

        let nft_transfer_ix = spl_token::instruction::transfer_checked(
            &spl_token::id(),
            &escrow_list_vault_account_pubkey,
            mint_account_info.key,
            token_account_info.key,
            &escrow_list_state_account_pubkey,
            &[&escrow_list_state_account_pubkey],
            1,
            0
        )?;
        
        invoke_signed(
            &nft_transfer_ix,
            &[
                token_program_info.clone(),
                escrow_list_vault_account_info.clone(),
                mint_account_info.clone(),
                token_account_info.clone(),
                escrow_list_state_account_info.clone(),
            ],

            &[&[
                mint_account_info.key.as_ref(),
                lister_account_info.key.as_ref(),
                &b"List"[..],
                &b"State"[..],
                &[nonce1]
            ]]
        )?;

        let close_ix = spl_token::instruction::close_account(
            &spl_token::id(), 
            &escrow_list_vault_account_pubkey, 
            &escrow_list_state_account_pubkey, 
            &escrow_list_state_account_pubkey, 
            &[&escrow_list_state_account_pubkey]
        )?;

        invoke_signed(
            &close_ix,
            &[
                token_program_info.clone(),
                escrow_list_vault_account_info.clone(),
                signer_info.clone(),
                escrow_list_state_account_info.clone(),
            ],
            &[&[
                mint_account_info.key.as_ref(),
                lister_account_info.key.as_ref(),
                &b"List"[..],
                &b"State"[..],
                &[nonce1]
            ]],
        )?;

        let platform_state = PlatformState::unpack_unchecked(&state_account_info.data.borrow())?;

        let mut list_state = ListEscrowState::unpack_unchecked(&escrow_list_state_account_info.data.borrow())?;
        if !list_state.lister.eq(lister_account_info.key) {
            return Err(ProgramError::InvalidAccountData);
        }
        if list_state.success {
            return Err(NFTError::ListingAlreadyFullfilled.into());
        }

        let fees_to_platform = (list_state.amount * platform_state.platform_fee ) / 1000000000;
        let amount_after_fees = list_state.amount - fees_to_platform;

        let transfer_lamports_lister_ix = system_instruction::transfer(signer_info.key, lister_account_info.key, amount_after_fees);
        invoke(
            &transfer_lamports_lister_ix,
            &[
                signer_info.clone(),
                lister_account_info.clone(),
                system_program_info.clone()
            ]
        )?;
        let transfer_lamports_platform_ix = system_instruction::transfer(signer_info.key, &vault_account_pubkey, fees_to_platform);
        invoke(
            &transfer_lamports_platform_ix,
            &[
                signer_info.clone(),
                vault_account_info.clone(),
                system_program_info.clone()
            ]
        )?;
        

        list_state.success = true;
        list_state.successful_buyer = *signer_info.key;

        ListEscrowState::pack(
            list_state,
            &mut escrow_list_state_account_info.data.borrow_mut()
        )?;

        let lamports = escrow_list_state_account_info.lamports();
        **escrow_list_state_account_info.try_borrow_mut_lamports()? = 0;
        **lister_account_info.try_borrow_mut_lamports()? += lamports;

        let log = format!("{{action: \"AcceptListing\", bidder: \"{}\", lister: \"{}\", amount: {}, mint: \"{}\"}}", signer_info.key.to_string(), lister_account_info.key.to_string(), list_state.amount, mint_account_info.key.to_string());
        msg!(&log);

        Ok(())
    }

    fn process_refund(
        accounts: &[AccountInfo],
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let signer_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let bidder_account_info = next_account_info(account_info_iter)?;
        let state_account_info = next_account_info(account_info_iter)?;
        let escrow_bid_state_account_info = next_account_info(account_info_iter)?;
        let escrow_bid_vault_account_info = next_account_info(account_info_iter)?;

        if !signer_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if !(mint_account_info.owner.eq(&spl_token::id())) {
            return Err(ProgramError::InvalidAccountData);
        }

        let (state_account_pubkey, _) = Pubkey::find_program_address(&[b"Platform", b"State"], program_id);
        if !(state_account_info.key.eq(&state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }
        let state_info = PlatformState::unpack_unchecked(&state_account_info.data.borrow())?;
        if !state_info.is_initialized(){
            return Err(ProgramError::UninitializedAccount);
        }
        if !state_info.authority.eq(signer_info.key) {
            return Err(NFTError::InvalidAuthority.into()); 
        }

        let (escrow_bid_state_account_pubkey, _) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            bidder_account_info.key.as_ref(),
            b"Bid",
            b"State"
            ],
            program_id
        );
        if !(escrow_bid_state_account_info.key.eq(&escrow_bid_state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }
        let (escrow_bid_vault_account_pubkey, _) = Pubkey::find_program_address(&[
            mint_account_info.key.as_ref(),
            bidder_account_info.key.as_ref(),
            b"Bid",
            b"Vault"
            ],
            program_id
        );
        if !(escrow_bid_vault_account_info.key.eq(&escrow_bid_vault_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let lamports = escrow_bid_state_account_info.lamports() + escrow_bid_vault_account_info.lamports();
        **escrow_bid_state_account_info.try_borrow_mut_lamports()? = 0;
        **escrow_bid_vault_account_info.try_borrow_mut_lamports()? = 0;
        **bidder_account_info.try_borrow_mut_lamports()? += lamports;

        Ok(())
    }
}
