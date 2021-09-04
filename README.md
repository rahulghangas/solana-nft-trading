# Solana-programs

## Deployer
To deploy and set authority, have a default keypair associated with your solana config

## Setup
Mock bidder and lister keypairs are already present, but you need to
  1. Fund them
  2. Create a token
  3. Create token accounts for both users and send tokens accordingly

This can be done as follows
  #### Fund users
  * `solana airdrop $ROOT_PATH$/dist/bidder/bidder-keypair.json`
  * `solana airdrop $ROOT_PATH$/dist/lister/lister-keypair.json`
  
  #### Create a token
  * `spl-token create-token --decimals 0` (assume created token address is `$TOKEN_MINT$` from hereon)
  
  #### Create and mint to user account(s)
  * `spl-token create-account $TOKEN_MINT$  --owner ~/repos/solana-nft-trading/dist/bidder/bidder-keypair.json`
  * `spl-token create-account $TOKEN_MINT$  --owner ~/repos/solana-nft-trading/dist/lister/lister-keypair.json` (assume created token account address is `$LISTER_TOKEN_ADDRESS$` from hereon)
  * `spl-token mint $TOKEN_MINT$ 1 $LISTER_TOKEN_ADDRESS$`
  
  #### Disable furthur mints (optional)
  * `spl-token authorize $TOKEN_MINT$ mint --disable`

  #### Build, deploy and initialize program
  * `npm run build:program-rust`
  * `solana program deploy $ROOT_PATH$/dist/program/test.so`
  * `npm run init`

  #### Edit client mint pubkey
  Go to `$ROOT_PATH/src/client/test.ts` and replace the top level `tokenMintPubKey` with your new token `$TOKEN_MINT`
  
## Trading
  #### Listing
    You can list the token created above by run `npm run list`
  
  #### Delisting
    You can delist the token created above by run `npm run delist`
  
  #### Bidding
    You can bid for the token created above by run `npm run bid`
  
  #### Withdraw Bid
    You can withdraw bid for the token created above by run `npm run withdraw-bid`
  
  #### Trading
    A trade can happen two ways (a lister accepts a bid, or a bidder accepts a listing)
    * Lister can accept bid usign `npm run accept-bid`
  
      --Note: Both a listing a bid should be present in order to call `AcceptBid`. `AcceptBid` is a non-inavsive instruction, that is, it doesn't assume that the bidder has a token accoutn yet. After a successful trade, the bidder can create a token account and call the `WithdrawOnSuccess` instruction. We have already set up the bidder account, so you can simply call call `npm run withdraw-on-success`
    
    * Bidder can accept lisitng using `npm run accept-listing`

      A bid doesn't nend to be placed to accept a listing as is. That is, a listing can be accepted at the designated listing price without setting up a bidding escrow

## Refund Users
The program authority can close escrows and refund bidders at will by calling the `RefundUser` instruction. Run `npm run refund` to refund the test bidder

## Misc Admin instruction
The admin can set a new authority or change platform fees using the `ChangeAuthority` and `ChangeFee` instructions
