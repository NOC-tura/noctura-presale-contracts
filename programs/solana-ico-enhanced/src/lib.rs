use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use anchor_spl::associated_token::AssociatedToken;
use pyth_solana_receiver_sdk::price_update::{PriceUpdateV2, get_feed_id_from_hex};

declare_id!("6nTTJwtDuxjv8C1JMsajYQapmPAGrC3QF1w5nu9LXJvt");

// =====================================================
// PHASE 1 ENHANCEMENTS - NOCTURA PRESALE & STAKING
// =====================================================

// Constants
pub const ICO_MINT_ADDRESS: &str = "DPE8QndfALRhjnJcNk6SRetcLqoYkWQ92Y5Dmg8yqBS2";
pub const USDT_MINT_ADDRESS: &str = "CgSYMf9CaxaLyBpeMQeQUogmZBEd1YJ2vJik9YaSxuDE";
pub const USDC_MINT_ADDRESS: &str = "zLUFPMH11VmqW7J711kryhDYhyhduqKBF8ZuMgPDk1q";

// Pyth Price Feed IDs (Devnet)
// SOL/USD: https://pyth.network/developers/price-feed-ids#solana-devnet
pub const PYTH_SOL_USD_FEED_ID: &str = "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";

pub const TOKEN_DECIMALS: u64 = 1_000_000_000; // 10^9 for SPL token decimals
pub const USDT_DECIMALS: u64 = 1_000_000; // USDT has 6 decimals
pub const USDC_DECIMALS: u64 = 1_000_000; // USDC has 6 decimals

pub const SECONDS_PER_DAY: i64 = 86400;
pub const SECONDS_PER_YEAR: i64 = 31536000;
pub const COOLDOWN_PERIOD: i64 = 172800; // 48 hours in seconds
pub const MAX_REFERRAL_PERCENTAGE: u64 = 20;
pub const REFERRAL_BONUS_PERCENTAGE: u64 = 10; // 10% for presale referrals

// Presale Constants (10 stages, prices in USD cents with 4 decimals)
pub const PRESALE_TOTAL_ALLOCATION: u64 = 102_400_000; // 40% of 256M supply
pub const TOKENS_PER_STAGE: u64 = 10_240_000;
pub const PRESALE_MIN_PURCHASE_USD: u64 = 2500; // $25.00 in cents
pub const PRESALE_MAX_PURCHASE_USD: u64 = 5_000_000; // $50,000 per transaction in cents
pub const PRESALE_MAX_TOTAL_PER_USER_USD: u64 = 20_000_000; // $200,000 maximum total per user in cents

// Community Rewards Pool (5% = 12.8M NOC) - for referral bonuses, airdrops, etc.
pub const COMMUNITY_REWARDS_ALLOCATION: u64 = 12_800_000; // 5% of 256M supply

// Stage prices in cents (4 decimal precision): $0.1501 = 1501 (representing $0.1501)
pub const STAGE_PRICES: [u64; 10] = [
    1501,  // Stage 1: $0.1501
    1723,  // Stage 2: $0.1723
    1945,  // Stage 3: $0.1945
    2167,  // Stage 4: $0.2167
    2389,  // Stage 5: $0.2389
    2611,  // Stage 6: $0.2611
    2833,  // Stage 7: $0.2833
    3055,  // Stage 8: $0.3055
    3277,  // Stage 9: $0.3277
    3499,  // Stage 10: $0.3499
];

// Enhanced APY rates for Phase 1 (matching whitepaper targets)
pub const APY_TIER_A: u64 = 128; // 365 days
pub const APY_TIER_B: u64 = 68;  // 182 days
pub const APY_TIER_C: u64 = 34;  // 90 days

// Tier lock periods
pub const LOCK_PERIOD_TIER_A: u64 = 365;
pub const LOCK_PERIOD_TIER_B: u64 = 182;
pub const LOCK_PERIOD_TIER_C: u64 = 90;

// Max stake per tier (to preserve high APYs)
pub const MAX_STAKE_TIER_A: u64 = 50_000_000 * TOKEN_DECIMALS; // 50M tokens max for TierA (highest APR)

// Global staking cap (20% of total supply = 51.2M tokens)
pub const MAX_TOTAL_STAKED: u64 = 51_200_000 * TOKEN_DECIMALS; // 51.2M tokens max total staking

// Team allocation (8% = 20.48M NOC) with 18-month lockup after TGE
pub const TEAM_ALLOCATION: u64 = 20_480_000; // 8% of 256M supply
pub const TEAM_LOCK_PERIOD_SECONDS: i64 = 18 * 30 * 86400; // 18 months in seconds (~547 days)

// Cross-chain purchase limits (security)
pub const CROSS_CHAIN_MIN_USD_CENTS: u64 = 2500; // $25 minimum per transaction
pub const CROSS_CHAIN_MAX_USD_CENTS: u64 = 5_000_000; // $50,000 maximum per transaction
pub const CROSS_CHAIN_MAX_TOTAL_USD_CENTS: u64 = 20_000_000; // $200,000 maximum per user total
pub const CROSS_CHAIN_COOLDOWN_SECONDS: i64 = 30; // 30 seconds between purchases

#[error_code]
pub enum ErrorCode {
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Invalid admin")]
    InvalidAdmin,
    #[msg("Address is blocked")]
    AddressBlocked,
    #[msg("Invalid price")]
    InvalidPrice,
    #[msg("Invalid ratio")]
    InvalidRatio,
    #[msg("Invalid address")]
    InvalidAddress,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Amount below minimum stake")]
    BelowMinimumStake,
    #[msg("Invalid lock period")]
    InvalidLockPeriod,
    #[msg("Stake not found")]
    StakeNotFound,
    #[msg("Stake not active")]
    StakeNotActive,
    #[msg("Still in lock period")]
    StillInLockPeriod,
    #[msg("Not stake owner")]
    NotStakeOwner,
    #[msg("No rewards to harvest")]
    NoRewards,
    #[msg("Invalid referrer address")]
    InvalidReferrer,
    #[msg("Cannot refer yourself")]
    CannotReferYourself,
    #[msg("Already registered with a referrer")]
    AlreadyHasReferrer,
    #[msg("Referral percentage too high")]
    ReferralPercentageTooHigh,
    #[msg("Insufficient token balance")]
    InsufficientBalance,
    #[msg("Cannot withdraw staked tokens")]
    CannotWithdrawStakedTokens,
    // Phase 1 New Errors
    #[msg("Presale not started yet")]
    PresaleNotStarted,
    #[msg("Presale has ended")]
    PresaleEnded,
    #[msg("Purchase below minimum")]
    BelowMinimumPurchase,
    #[msg("Purchase exceeds maximum per user")]
    ExceedsMaximumPurchase,
    #[msg("Presale hard cap reached")]
    PresaleHardCapReached,
    #[msg("Still in cooldown period")]
    StillInCooldown,
    #[msg("Tokens not yet unlocked (pre-TGE)")]
    TokensLocked,
    #[msg("Allocation already claimed")]
    AllocationAlreadyClaimed,
    #[msg("Tier A is full")]
    TierAFull,
    #[msg("Global staking cap reached (51.2M NOC)")]
    StakingCapReached,
    #[msg("Referral depth exceeded")]
    ReferralDepthExceeded,
    #[msg("Invalid PDA derivation")]
    InvalidPDA,
    #[msg("Referral rewards pool exhausted")]
    ReferralPoolExhausted,
    // Cross-chain errors
    #[msg("Invalid coordinator")]
    InvalidCoordinator,
    #[msg("Cross-chain allocation already exists")]
    CrossChainAllocationExists,
    #[msg("Invalid chain ID")]
    InvalidChainId,
    #[msg("Transaction already processed")]
    TransactionAlreadyProcessed,
    // Vesting staking errors
    #[msg("Vesting stake cannot be unstaked before TGE")]
    VestingStakeLocked,
    // Team vesting errors
    #[msg("Team tokens are still locked (18 months after TGE)")]
    TeamTokensLocked,
    #[msg("Team allocation already initialized")]
    TeamAllocationExists,
    #[msg("Team allocation not found")]
    TeamAllocationNotFound,
    #[msg("No team tokens to claim")]
    NoTeamTokensToClaim,
    #[msg("Exceeds team allocation limit")]
    ExceedsTeamAllocation,
    // Cross-chain security errors
    #[msg("Cross-chain purchase below minimum ($10)")]
    CrossChainBelowMinimum,
    #[msg("Cross-chain purchase exceeds maximum ($10,000)")]
    CrossChainExceedsMaximum,
    #[msg("Cross-chain user total exceeds limit ($50,000)")]
    CrossChainUserLimitExceeded,
    #[msg("Cross-chain purchase cooldown active (30s)")]
    CrossChainCooldown,
}

#[program]
pub mod ico {
    use super::*;

    // =====================================================
    // INITIALIZATION & ADMIN FUNCTIONS
    // =====================================================

    pub fn initialize(ctx: Context<Initialize>, ico_amount: u64, tge_timestamp: i64) -> Result<()> {
        let raw_amount = ico_amount
            .checked_mul(TOKEN_DECIMALS)
            .ok_or(ErrorCode::Overflow)?;

        // Transfer ICO tokens to program ATA (these will be distributed at TGE)
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.ico_ata_for_admin.to_account_info(),
                to: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
                authority: ctx.accounts.admin.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, raw_amount)?;

        let clock = Clock::get()?;
        let config = &mut ctx.accounts.config;
        config.admin = ctx.accounts.admin.key();
        config.sale_token = ctx.accounts.ico_mint.key();
        config.usdt_address = Pubkey::default();
        config.usdc_address = Pubkey::default();
        
        // Presale pricing (will be updated via oracle for SOL/stable conversions)
        config.sol_price_for_token = 1_000_000; // 0.001 SOL in lamports
        config.sol_price_for_stablecoin = 1_000_000;
        config.usdt_ratio = 20;
        config.usdc_ratio = 20;
        
        // Presale state
        config.current_stage = 0; // Stages 0-9 for array indexing
        config.stage_tokens_sold = 0;
        config.tokens_sold = 0;
        config.total_usd_raised_cents = 0;
        config.presale_start_time = clock.unix_timestamp;
        config.tge_timestamp = tge_timestamp;
        config.presale_active = true;
        
        // Staking state
        config.total_penalty_collected = 0;
        config.min_stake_amount = 100 * TOKEN_DECIMALS;
        config.total_staked = 0;
        config.total_staked_tier_a = 0;
        config.total_rewards_distributed = 0;
        config.total_stakers = 0;
        config.next_stake_id = 1;
        config.referral_reward_percentage = REFERRAL_BONUS_PERCENTAGE;
        config.total_referral_bonuses = 0;
        
        // Cross-chain state
        config.coordinator = Pubkey::default(); // Set via set_coordinator()
        config.cross_chain_tokens_sold = 0;
        
        // Purchase limits (0 = use default constants)
        config.max_per_user_usd = 0; // Use PRESALE_MAX_PURCHASE_USD constant
        config.min_purchase_usd = 0; // Use PRESALE_MIN_PURCHASE_USD constant
        
        // SOL treasury - defaults to admin, can be changed to Squads vault
        config.sol_treasury = ctx.accounts.admin.key();

        msg!("ICO initialized with {} tokens, TGE at {}", ico_amount, tge_timestamp);
        Ok(())
    }

    // =====================================================
    // PHASE 1: ALLOCATION-ONLY PRESALE
    // =====================================================

    /// Purchase tokens during presale with SOL - stores ALLOCATION ONLY (no minting until TGE)
    /// Uses Pyth Oracle for real-time SOL/USD price
    pub fn presale_purchase_with_sol(
        ctx: Context<PresalePurchaseWithSol>,
        sol_amount: u64,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        let clock = Clock::get()?;

        // Validate presale is active
        require!(config.presale_active, ErrorCode::PresaleNotStarted);
        require!(
            clock.unix_timestamp >= config.presale_start_time,
            ErrorCode::PresaleNotStarted
        );
        require!(
            config.tokens_sold < PRESALE_TOTAL_ALLOCATION * TOKEN_DECIMALS,
            ErrorCode::PresaleHardCapReached
        );

        // Validate user not blocked
        require!(
            !ctx.accounts.user_account.is_blocked,
            ErrorCode::AddressBlocked
        );
        require!(sol_amount > 0, ErrorCode::InvalidAmount);

        // Get SOL/USD price from Pyth Oracle
        // SECURITY: Max staleness 120s for mainnet
        let price_update = &ctx.accounts.pyth_sol_usd_price;
        let price_data = price_update.get_price_no_older_than(
            &Clock::get()?,
            120, // Max age 120 seconds (2 minutes) - MAINNET
            &get_feed_id_from_hex(PYTH_SOL_USD_FEED_ID)?
        )?;

        // Calculate USD value
        // price_data.price is in format: price * 10^expo
        // For SOL/USD: expo = -8, so if SOL = $150.00, price = 15000000000
        let sol_price_usd = price_data.price; // e.g., 15000000000 for $150.00
        let expo = price_data.exponent; // e.g., -8
        
        // Convert lamports to SOL (divide by 1e9), multiply by price, adjust for exponent
        // usd_value = (sol_amount / 1e9) * (sol_price_usd * 10^expo)
        // usd_cents = usd_value * 100
        let usd_cents = (sol_amount as i128)
            .checked_mul(sol_price_usd as i128)
            .ok_or(ErrorCode::Overflow)?
            .checked_mul(100) // Convert to cents
            .ok_or(ErrorCode::Overflow)?;
        
        // Adjust for decimals: sol (9 decimals) and pyth exponent
        let divisor = 10_i128.pow((9_i32 - expo) as u32);
        let usd_cents = (usd_cents / divisor) as u64;

        msg!("SOL amount: {} lamports, SOL price: ${} (expo: {}), USD value: {} cents", 
            sol_amount, sol_price_usd, expo, usd_cents);

        // Validate purchase limits (use config values if set, otherwise use constants)
        let min_purchase = if config.min_purchase_usd > 0 { config.min_purchase_usd } else { PRESALE_MIN_PURCHASE_USD };
        let max_purchase = if config.max_per_user_usd > 0 { config.max_per_user_usd } else { PRESALE_MAX_PURCHASE_USD };
        
        // Check minimum per transaction
        require!(usd_cents >= min_purchase, ErrorCode::BelowMinimumPurchase);
        
        // Check maximum per transaction ($10,000)
        require!(usd_cents <= max_purchase, ErrorCode::ExceedsMaximumPurchase);
        
        let user_allocation = &mut ctx.accounts.user_allocation;
        let new_total_spent = user_allocation.total_spent_cents
            .checked_add(usd_cents)
            .ok_or(ErrorCode::Overflow)?;
        
        // Check maximum total per user ($25,600)
        require!(
            new_total_spent <= PRESALE_MAX_TOTAL_PER_USER_USD,
            ErrorCode::ExceedsMaximumPurchase
        );

        // Calculate tokens based on current stage price
        let tokens_to_allocate = calculate_tokens_for_usd(usd_cents, config.current_stage)?;

        // === CEI PATTERN: EFFECTS FIRST, then INTERACTIONS ===
        
        // Process referral bonus (10%) - FROM COMMUNITY REWARDS POOL (not presale)
        // ONE-TIME ONLY: Referral bonus is only given on the FIRST purchase
        let referral_bonus = if user_allocation.referrer != Pubkey::default() && user_allocation.purchase_count == 0 {
            let bonus = tokens_to_allocate
                .checked_mul(REFERRAL_BONUS_PERCENTAGE)
                .ok_or(ErrorCode::Overflow)?
                .checked_div(100)
                .ok_or(ErrorCode::Overflow)?;

            // Check if Community Rewards pool has enough tokens
            let new_total_referral = config.total_referral_bonuses
                .checked_add(bonus)
                .ok_or(ErrorCode::Overflow)?;
            
            if new_total_referral > COMMUNITY_REWARDS_ALLOCATION * TOKEN_DECIMALS {
                // Pool exhausted - no bonus but purchase continues
                msg!("Referral pool exhausted - no bonus awarded");
                0
            } else {
                // Update referrer's allocation (if provided - check if account is initialized)
                if ctx.accounts.referrer_allocation.data_is_empty() {
                    // No referrer - skip bonus
                    msg!("No referrer provided");
                    0
                } else {
                    // SECURITY: Validate referrer_allocation PDA matches expected seeds
                    let referrer_pubkey = user_allocation.referrer;
                    let (expected_pda, _bump) = Pubkey::find_program_address(
                        &[b"allocation", referrer_pubkey.as_ref()],
                        ctx.program_id
                    );
                    require!(
                        ctx.accounts.referrer_allocation.key() == expected_pda,
                        ErrorCode::InvalidPDA
                    );
                    
                    // Try to deserialize referrer allocation
                    let mut referrer_data = ctx.accounts.referrer_allocation.try_borrow_mut_data()?;
                    let mut referrer_alloc = PresaleAllocation::try_deserialize(&mut &referrer_data[..])?;
                    
                    referrer_alloc.referral_bonus_tokens = referrer_alloc
                        .referral_bonus_tokens
                        .checked_add(bonus)
                        .ok_or(ErrorCode::Overflow)?;
                    referrer_alloc.total_tokens = referrer_alloc
                        .total_tokens
                        .checked_add(bonus)
                        .ok_or(ErrorCode::Overflow)?;
                    
                    // Serialize back
                    referrer_alloc.try_serialize(&mut &mut referrer_data[..])?;
                    
                    // Track total referral bonuses issued (from Community Rewards pool)
                    config.total_referral_bonuses = new_total_referral;
                    
                    msg!("One-time referral bonus awarded: {} tokens", bonus);
                    bonus
                }
            }
        } else {
            if user_allocation.purchase_count > 0 {
                msg!("Referral bonus skipped - not first purchase");
            }
            0
        };

        // Update user allocation (NO MINTING - just record keeping)
        user_allocation.user = ctx.accounts.user.key();
        user_allocation.total_tokens = user_allocation
            .total_tokens
            .checked_add(tokens_to_allocate)
            .ok_or(ErrorCode::Overflow)?;
        user_allocation.total_spent_cents = new_total_spent;
        user_allocation.purchase_count = user_allocation.purchase_count.checked_add(1).ok_or(ErrorCode::Overflow)?;
        
        if user_allocation.purchase_count == 1 {
            user_allocation.first_purchase_at = clock.unix_timestamp;
        }
        user_allocation.last_purchase_at = clock.unix_timestamp;

        // Update stage progress
        config.stage_tokens_sold = config
            .stage_tokens_sold
            .checked_add(tokens_to_allocate)
            .ok_or(ErrorCode::Overflow)?;
        config.tokens_sold = config
            .tokens_sold
            .checked_add(tokens_to_allocate)
            .ok_or(ErrorCode::Overflow)?;
        config.total_usd_raised_cents = config
            .total_usd_raised_cents
            .checked_add(usd_cents)
            .ok_or(ErrorCode::Overflow)?;

        // Advance stage if current stage is full
        while config.stage_tokens_sold >= TOKENS_PER_STAGE * TOKEN_DECIMALS && config.current_stage < 9 {
            config.current_stage += 1;
            config.stage_tokens_sold = config.stage_tokens_sold
                .checked_sub(TOKENS_PER_STAGE * TOKEN_DECIMALS)
                .ok_or(ErrorCode::Overflow)?;
            
            msg!("Advanced to stage {}", config.current_stage + 1);
        }

        // Close presale if hard cap reached
        if config.tokens_sold >= PRESALE_TOTAL_ALLOCATION * TOKEN_DECIMALS {
            config.presale_active = false;
            msg!("Presale hard cap reached - presale closed");
        }

        // === CEI PATTERN: INTERACTIONS LAST ===
        // Transfer SOL payment to treasury (AFTER all state updates)
        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.sol_treasury.key(),
            sol_amount,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.sol_treasury.to_account_info(),
            ],
        )?;

        msg!(
            "Allocation recorded: {} tokens to user, {} referral bonus",
            tokens_to_allocate,
            referral_bonus
        );

        Ok(())
    }

    /// Purchase tokens during presale with USDT - stores ALLOCATION ONLY
    /// USDT is pegged to $1.00 USD (no oracle needed)
    pub fn presale_purchase_with_usdt(
        ctx: Context<PresalePurchaseWithStablecoin>,
        usdt_amount: u64,
    ) -> Result<()> {
        presale_purchase_with_stablecoin_internal(
            &mut ctx.accounts.config,
            &mut ctx.accounts.user_account,
            &mut ctx.accounts.user_allocation,
            &ctx.accounts.referrer_allocation,
            &ctx.accounts.stablecoin_ata_for_user,
            &ctx.accounts.stablecoin_ata_for_admin,
            &ctx.accounts.stablecoin_mint,
            &ctx.accounts.user,
            &ctx.accounts.token_program,
            usdt_amount,
            "USDT"
        )
    }

    /// Purchase tokens during presale with USDC - stores ALLOCATION ONLY
    /// USDC is pegged to $1.00 USD (no oracle needed)
    pub fn presale_purchase_with_usdc(
        ctx: Context<PresalePurchaseWithStablecoin>,
        usdc_amount: u64,
    ) -> Result<()> {
        presale_purchase_with_stablecoin_internal(
            &mut ctx.accounts.config,
            &mut ctx.accounts.user_account,
            &mut ctx.accounts.user_allocation,
            &ctx.accounts.referrer_allocation,
            &ctx.accounts.stablecoin_ata_for_user,
            &ctx.accounts.stablecoin_ata_for_admin,
            &ctx.accounts.stablecoin_mint,
            &ctx.accounts.user,
            &ctx.accounts.token_program,
            usdc_amount,
            "USDC"
        )
    }

    /// Purchase tokens during presale with USDT and automatically stake them
    /// USDT is pegged to $1.00 USD (no oracle needed)
    /// Uses PDA per user per tier (max 3 stake accounts per user)
    pub fn presale_purchase_usdt_and_vest_stake(
        ctx: Context<PresalePurchaseStablecoinAndVestStake>,
        usdt_amount: u64,
        tier: StakeTier,
        auto_compound: bool,
    ) -> Result<()> {
        presale_purchase_stablecoin_and_vest_stake_internal(
            &mut ctx.accounts.config,
            &mut ctx.accounts.user_account,
            &mut ctx.accounts.user_allocation,
            &mut ctx.accounts.stake_account,
            &ctx.accounts.stablecoin_ata_for_user,
            &ctx.accounts.stablecoin_ata_for_admin,
            &ctx.accounts.user,
            &ctx.accounts.token_program,
            usdt_amount,
            tier,
            auto_compound,
            "USDT"
        )
    }

    /// Purchase tokens during presale with USDC and automatically stake them
    /// USDC is pegged to $1.00 USD (no oracle needed)
    /// Uses PDA per user per tier (max 3 stake accounts per user)
    pub fn presale_purchase_usdc_and_vest_stake(
        ctx: Context<PresalePurchaseStablecoinAndVestStake>,
        usdc_amount: u64,
        tier: StakeTier,
        auto_compound: bool,
    ) -> Result<()> {
        presale_purchase_stablecoin_and_vest_stake_internal(
            &mut ctx.accounts.config,
            &mut ctx.accounts.user_account,
            &mut ctx.accounts.user_allocation,
            &mut ctx.accounts.stake_account,
            &ctx.accounts.stablecoin_ata_for_user,
            &ctx.accounts.stablecoin_ata_for_admin,
            &ctx.accounts.user,
            &ctx.accounts.token_program,
            usdc_amount,
            tier,
            auto_compound,
            "USDC"
        )
    }

    /// Claim presale allocation at/after TGE
    pub fn claim_presale_allocation(ctx: Context<ClaimPresaleAllocation>) -> Result<()> {
        let config = &ctx.accounts.config;
        let clock = Clock::get()?;
        let user_allocation = &mut ctx.accounts.user_allocation;

        // === CHECKS ===
        // Validate TGE has occurred
        require!(
            clock.unix_timestamp >= config.tge_timestamp,
            ErrorCode::TokensLocked
        );

        // Validate allocation exists and not claimed
        require!(user_allocation.total_tokens > 0, ErrorCode::InvalidAmount);
        require!(!user_allocation.claimed, ErrorCode::AllocationAlreadyClaimed);

        // Store tokens to transfer (need before state change)
        let tokens_to_claim = user_allocation.total_tokens;

        // === EFFECTS (state changes first) ===
        user_allocation.claimed = true;

        // === INTERACTIONS (external calls last) ===
        // NOW mint the tokens (first time tokens actually exist)
        let ico_mint_key = ctx.accounts.ico_mint.key();
        let (expected_pda, bump) = Pubkey::find_program_address(
            &[ico_mint_key.as_ref()],
            ctx.program_id,
        );
        
        require!(
            ctx.accounts.ico_ata_for_ico_program.key() == expected_pda,
            ErrorCode::InvalidPDA
        );
        
        let seeds = &[ico_mint_key.as_ref(), &[bump]];
        let signer = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
                to: ctx.accounts.ico_ata_for_user.to_account_info(),
                authority: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
            },
            signer,
        );
        token::transfer(cpi_ctx, tokens_to_claim)?;

        msg!(
            "Claimed {} tokens for user {}",
            tokens_to_claim,
            ctx.accounts.user.key()
        );

        Ok(())
    }

    /// Admin claim tokens for a user (when user cannot claim themselves)
    /// Only admin can call this - transfers from vault to user's wallet
    /// Sets claimed = true on user's allocation for proper tracking
    pub fn admin_claim_for_user(ctx: Context<AdminClaimForUser>) -> Result<()> {
        let config = &ctx.accounts.config;
        let clock = Clock::get()?;
        let user_allocation = &mut ctx.accounts.user_allocation;

        // === CHECKS ===
        // Validate TGE has occurred
        require!(
            clock.unix_timestamp >= config.tge_timestamp,
            ErrorCode::TokensLocked
        );

        // Validate allocation exists and not already claimed
        require!(user_allocation.total_tokens > 0, ErrorCode::InvalidAmount);
        require!(!user_allocation.claimed, ErrorCode::AllocationAlreadyClaimed);

        // Store tokens to transfer (need before state change)
        let tokens_to_claim = user_allocation.total_tokens;

        // === EFFECTS (state changes first) ===
        user_allocation.claimed = true;

        // === INTERACTIONS (external calls last) ===
        // Transfer tokens from vault to user
        let ico_mint_key = ctx.accounts.ico_mint.key();
        let (expected_pda, bump) = Pubkey::find_program_address(
            &[ico_mint_key.as_ref()],
            ctx.program_id,
        );
        
        require!(
            ctx.accounts.ico_ata_for_ico_program.key() == expected_pda,
            ErrorCode::InvalidPDA
        );
        
        let seeds = &[ico_mint_key.as_ref(), &[bump]];
        let signer = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
                to: ctx.accounts.ico_ata_for_user.to_account_info(),
                authority: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
            },
            signer,
        );
        token::transfer(cpi_ctx, tokens_to_claim)?;

        msg!(
            "ADMIN_CLAIM: Admin {} claimed {} tokens for user {}",
            ctx.accounts.admin.key(),
            tokens_to_claim,
            ctx.accounts.user.key()
        );

        Ok(())
    }

    /// Admin function to add allocation for giveaways/airdrops
    /// Does NOT modify existing PresaleAllocation struct - reuses it
    /// Only admin can call this - creates allocation without payment
    pub fn admin_add_allocation(
        ctx: Context<AdminAddAllocation>,
        token_amount: u64,
    ) -> Result<()> {
        let config = &ctx.accounts.config;

        // === SECURITY: Validate admin ===
        require!(
            ctx.accounts.admin.key() == config.admin,
            ErrorCode::InvalidAdmin
        );

        // === VALIDATION ===
        require!(token_amount > 0, ErrorCode::InvalidAmount);

        // Check giveaway cap (giveaways come from community rewards pool)
        // Community Rewards = 12.8M NOC (5%)
        let max_giveaway_total = COMMUNITY_REWARDS_ALLOCATION * TOKEN_DECIMALS;
        let new_total_giveaway = config.total_referral_bonuses
            .checked_add(token_amount)
            .ok_or(ErrorCode::Overflow)?;
        
        require!(
            new_total_giveaway <= max_giveaway_total,
            ErrorCode::PresaleHardCapReached
        );

        // === EFFECTS: Update or create allocation ===
        let user_allocation = &mut ctx.accounts.user_allocation;
        
        // Initialize if new allocation
        if user_allocation.user == Pubkey::default() {
            user_allocation.user = ctx.accounts.recipient.key();
            user_allocation.total_tokens = 0;
            user_allocation.total_spent_cents = 0;
            user_allocation.claimed = false;
            user_allocation.referrer = Pubkey::default();
            user_allocation.referral_bonus_tokens = 0;
            user_allocation.purchase_count = 0;
            user_allocation.first_purchase_at = 0;
            user_allocation.last_purchase_at = 0;
        }

        // Add tokens to allocation
        user_allocation.total_tokens = user_allocation
            .total_tokens
            .checked_add(token_amount)
            .ok_or(ErrorCode::Overflow)?;
        
        // Track as referral bonus (reuse existing field for giveaway tracking)
        user_allocation.referral_bonus_tokens = user_allocation
            .referral_bonus_tokens
            .checked_add(token_amount)
            .ok_or(ErrorCode::Overflow)?;

        // Update config to track total giveaways issued
        let config = &mut ctx.accounts.config;
        config.total_referral_bonuses = new_total_giveaway;

        msg!(
            "ADMIN_GIVEAWAY: Added {} tokens to user {} (total allocation: {})",
            token_amount,
            ctx.accounts.recipient.key(),
            user_allocation.total_tokens
        );

        Ok(())
    }

    /// Claim presale allocation and immediately stake with chosen tier
    pub fn claim_and_stake(
        ctx: Context<ClaimAndStake>,
        tier: StakeTier,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        let clock = Clock::get()?;
        let user_allocation = &mut ctx.accounts.user_allocation;

        // Validate TGE has occurred
        require!(
            clock.unix_timestamp >= config.tge_timestamp,
            ErrorCode::TokensLocked
        );

        // Validate not already claimed
        require!(!user_allocation.claimed, ErrorCode::AllocationAlreadyClaimed);
        require!(user_allocation.total_tokens > 0, ErrorCode::InvalidAmount);

        let amount = user_allocation.total_tokens;

        // Check global staking cap
        require!(
            config.total_staked.checked_add(amount).ok_or(ErrorCode::Overflow)? <= MAX_TOTAL_STAKED,
            ErrorCode::StakingCapReached
        );

        // Validate tier-specific constraints
        if tier == StakeTier::TierA {
            require!(
                config.total_staked_tier_a.checked_add(amount).ok_or(ErrorCode::Overflow)? <= MAX_STAKE_TIER_A,
                ErrorCode::TierAFull
            );
        }

        // Mark as claimed
        user_allocation.claimed = true;

        // Transfer tokens to staking vault (NOT to user wallet)
        let ico_mint_key = ctx.accounts.ico_mint.key();
        let (expected_pda, bump) = Pubkey::find_program_address(
            &[ico_mint_key.as_ref()],
            ctx.program_id,
        );
        
        require!(
            ctx.accounts.ico_ata_for_ico_program.key() == expected_pda,
            ErrorCode::InvalidPDA
        );
        
        let seeds = &[ico_mint_key.as_ref(), &[bump]];
        let signer = &[&seeds[..]];

        let _cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
                to: ctx.accounts.ico_ata_for_ico_program.to_account_info(), // Stays in vault
                authority: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
            },
            signer,
        );
        // Note: In real implementation, this transfer is optimized away since tokens stay in vault

        let stake_id = config.next_stake_id;
        let lock_period_days = match tier {
            StakeTier::TierA => LOCK_PERIOD_TIER_A,
            StakeTier::TierB => LOCK_PERIOD_TIER_B,
            StakeTier::TierC => LOCK_PERIOD_TIER_C,
        };

        // Create stake record
        let stake_account = &mut ctx.accounts.stake_account;
        stake_account.stake_id = stake_id;
        stake_account.owner = ctx.accounts.user.key();
        stake_account.amount = amount;
        stake_account.start_time = clock.unix_timestamp;
        stake_account.lock_period_days = lock_period_days;
        stake_account.last_reward_calculation = clock.unix_timestamp;
        stake_account.pending_rewards = 0;
        stake_account.active = true;
        stake_account.tier = tier;
        stake_account.auto_compound = false;
        stake_account.cooldown_start = 0;

        // Update config
        config.next_stake_id = stake_id.checked_add(1).ok_or(ErrorCode::Overflow)?;
        config.total_staked = config
            .total_staked
            .checked_add(amount)
            .ok_or(ErrorCode::Overflow)?;

        if tier == StakeTier::TierA {
            config.total_staked_tier_a = config
                .total_staked_tier_a
                .checked_add(amount)
                .ok_or(ErrorCode::Overflow)?;
        }

        if !ctx.accounts.user_account.has_staked {
            ctx.accounts.user_account.has_staked = true;
            config.total_stakers = config
                .total_stakers
                .checked_add(1)
                .ok_or(ErrorCode::Overflow)?;
        }

        msg!(
            "Claimed and staked {} tokens in {:?} for {} days",
            amount,
            tier,
            lock_period_days
        );

        Ok(())
    }

    // =====================================================
    // REFERRAL SYSTEM (Enhanced for Phase 1)
    // =====================================================

    pub fn register_referrer(ctx: Context<RegisterReferrer>, referrer: Pubkey) -> Result<()> {
        require!(
            !ctx.accounts.user_account.is_blocked,
            ErrorCode::AddressBlocked
        );
        require!(referrer != Pubkey::default(), ErrorCode::InvalidReferrer);
        require!(
            referrer != ctx.accounts.user.key(),
            ErrorCode::CannotReferYourself
        );

        let user_allocation = &mut ctx.accounts.user_allocation;
        require!(
            user_allocation.referrer == Pubkey::default(),
            ErrorCode::AlreadyHasReferrer
        );

        // Check referral depth to prevent loops (max 3 levels)
        // In production, implement full cycle detection
        
        user_allocation.referrer = referrer;
        msg!("Registered referrer for user");
        Ok(())
    }

    // =====================================================
    // VESTING STAKING - PRESALE WITH AUTOMATIC STAKING
    // Tokens are minted and immediately staked (locked until TGE)
    // =====================================================

    /// Purchase tokens during presale with SOL and automatically stake them
    /// Tokens are minted to staking pool and locked until TGE
    /// User earns staking rewards during presale period
    pub fn presale_purchase_and_vest_stake(
        ctx: Context<PresalePurchaseAndVestStake>,
        sol_amount: u64,
        tier: StakeTier,
        auto_compound: bool,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        let clock = Clock::get()?;

        // Validate presale is active
        require!(config.presale_active, ErrorCode::PresaleNotStarted);
        require!(
            clock.unix_timestamp >= config.presale_start_time,
            ErrorCode::PresaleNotStarted
        );
        require!(
            config.tokens_sold < PRESALE_TOTAL_ALLOCATION * TOKEN_DECIMALS,
            ErrorCode::PresaleHardCapReached
        );

        // Validate user not blocked
        require!(
            !ctx.accounts.user_account.is_blocked,
            ErrorCode::AddressBlocked
        );
        require!(sol_amount > 0, ErrorCode::InvalidAmount);

        // Get SOL/USD price from Pyth Oracle
        // SECURITY: Max staleness 120s for mainnet
        let price_update = &ctx.accounts.pyth_sol_usd_price;
        let price_data = price_update.get_price_no_older_than(
            &Clock::get()?,
            120, // Max age 120 seconds (2 minutes) - MAINNET
            &get_feed_id_from_hex(PYTH_SOL_USD_FEED_ID)?
        )?;

        let sol_price_usd = price_data.price;
        let expo = price_data.exponent;
        
        let usd_cents = (sol_amount as i128)
            .checked_mul(sol_price_usd as i128)
            .ok_or(ErrorCode::Overflow)?
            .checked_mul(100)
            .ok_or(ErrorCode::Overflow)?;
        
        let divisor = 10_i128.pow((9_i32 - expo) as u32);
        let usd_cents = (usd_cents / divisor) as u64;

        msg!("Vesting Stake: SOL amount: {} lamports, USD value: {} cents", sol_amount, usd_cents);

        // Validate purchase limits (use config values if set, otherwise use constants)
        let min_purchase = if config.min_purchase_usd > 0 { config.min_purchase_usd } else { PRESALE_MIN_PURCHASE_USD };
        let max_purchase = if config.max_per_user_usd > 0 { config.max_per_user_usd } else { PRESALE_MAX_PURCHASE_USD };
        
        // Check minimum per transaction
        require!(usd_cents >= min_purchase, ErrorCode::BelowMinimumPurchase);
        
        // Check maximum per transaction ($10,000)
        require!(usd_cents <= max_purchase, ErrorCode::ExceedsMaximumPurchase);
        
        let user_allocation = &mut ctx.accounts.user_allocation;
        let new_total_spent = user_allocation.total_spent_cents
            .checked_add(usd_cents)
            .ok_or(ErrorCode::Overflow)?;
        
        // Check maximum total per user ($25,600)
        require!(
            new_total_spent <= PRESALE_MAX_TOTAL_PER_USER_USD,
            ErrorCode::ExceedsMaximumPurchase
        );

        // Calculate tokens based on current stage price
        let tokens_to_stake = calculate_tokens_for_usd(usd_cents, config.current_stage)?;

        // Check global staking cap
        require!(
            config.total_staked.checked_add(tokens_to_stake).ok_or(ErrorCode::Overflow)? <= MAX_TOTAL_STAKED,
            ErrorCode::StakingCapReached
        );

        // Validate tier-specific constraints
        let lock_period_days = match tier {
            StakeTier::TierA => {
                require!(
                    config.total_staked_tier_a.checked_add(tokens_to_stake).ok_or(ErrorCode::Overflow)? <= MAX_STAKE_TIER_A,
                    ErrorCode::TierAFull
                );
                LOCK_PERIOD_TIER_A
            }
            StakeTier::TierB => LOCK_PERIOD_TIER_B,
            StakeTier::TierC => LOCK_PERIOD_TIER_C,
        };

        // === CEI PATTERN: EFFECTS FIRST ===
        // Transfer tokens from program vault to program vault (they stay in vault for staking)
        // The tokens are already in the program's ATA, we just need to record the stake

        // Update user allocation (record keeping)
        user_allocation.user = ctx.accounts.user.key();
        user_allocation.total_tokens = user_allocation
            .total_tokens
            .checked_add(tokens_to_stake)
            .ok_or(ErrorCode::Overflow)?;
        user_allocation.total_spent_cents = new_total_spent;
        user_allocation.purchase_count = user_allocation.purchase_count.checked_add(1).ok_or(ErrorCode::Overflow)?;
        
        if user_allocation.purchase_count == 1 {
            user_allocation.first_purchase_at = clock.unix_timestamp;
        }
        user_allocation.last_purchase_at = clock.unix_timestamp;

        // Create or update vesting stake (PDA per user per tier)
        let stake_account = &mut ctx.accounts.stake_account;
        let is_new_stake = stake_account.owner == Pubkey::default();
        
        if is_new_stake {
            // New stake account - initialize
            let stake_id = config.next_stake_id;
            stake_account.stake_id = stake_id;
            stake_account.owner = ctx.accounts.user.key();
            stake_account.amount = tokens_to_stake;
            stake_account.start_time = clock.unix_timestamp;
            stake_account.lock_period_days = lock_period_days;
            stake_account.last_reward_calculation = clock.unix_timestamp;
            stake_account.pending_rewards = 0;
            stake_account.active = true;
            stake_account.tier = tier;
            stake_account.auto_compound = auto_compound;
            stake_account.cooldown_start = 0;
            stake_account.is_vesting = true;
            stake_account.total_added = tokens_to_stake;
            
            config.next_stake_id = stake_id.checked_add(1).ok_or(ErrorCode::Overflow)?;
            
            msg!("Created new vesting stake: {} tokens in {:?} tier", tokens_to_stake, tier);
        } else {
            // Existing stake - calculate pending rewards first, then add new tokens
            let apy = match stake_account.tier {
                StakeTier::TierA => APY_TIER_A,
                StakeTier::TierB => APY_TIER_B,
                StakeTier::TierC => APY_TIER_C,
            };
            
            let time_elapsed = clock.unix_timestamp
                .checked_sub(stake_account.last_reward_calculation)
                .ok_or(ErrorCode::Overflow)?;
            
            if time_elapsed > 0 {
                let rewards = (stake_account.amount as u128)
                    .checked_mul(apy as u128)
                    .ok_or(ErrorCode::Overflow)?
                    .checked_mul(time_elapsed as u128)
                    .ok_or(ErrorCode::Overflow)?
                    .checked_div(100 * SECONDS_PER_YEAR as u128)
                    .ok_or(ErrorCode::Overflow)? as u64;
                
                if stake_account.auto_compound {
                    stake_account.amount = stake_account.amount
                        .checked_add(rewards)
                        .ok_or(ErrorCode::Overflow)?;
                } else {
                    stake_account.pending_rewards = stake_account.pending_rewards
                        .checked_add(rewards)
                        .ok_or(ErrorCode::Overflow)?;
                }
            }
            
            // Add new tokens to existing stake
            stake_account.amount = stake_account.amount
                .checked_add(tokens_to_stake)
                .ok_or(ErrorCode::Overflow)?;
            stake_account.total_added = stake_account.total_added
                .checked_add(tokens_to_stake)
                .ok_or(ErrorCode::Overflow)?;
            stake_account.last_reward_calculation = clock.unix_timestamp;
            stake_account.auto_compound = auto_compound; // Update auto_compound preference
            
            msg!("Added {} tokens to existing {:?} stake (total: {})", tokens_to_stake, tier, stake_account.amount);
        }

        // Update staking config
        config.total_staked = config
            .total_staked
            .checked_add(tokens_to_stake)
            .ok_or(ErrorCode::Overflow)?;

        if tier == StakeTier::TierA {
            config.total_staked_tier_a = config
                .total_staked_tier_a
                .checked_add(tokens_to_stake)
                .ok_or(ErrorCode::Overflow)?;
        }

        if !ctx.accounts.user_account.has_staked {
            ctx.accounts.user_account.has_staked = true;
            config.total_stakers = config
                .total_stakers
                .checked_add(1)
                .ok_or(ErrorCode::Overflow)?;
        }

        // Update presale stage progress
        config.stage_tokens_sold = config
            .stage_tokens_sold
            .checked_add(tokens_to_stake)
            .ok_or(ErrorCode::Overflow)?;
        config.tokens_sold = config
            .tokens_sold
            .checked_add(tokens_to_stake)
            .ok_or(ErrorCode::Overflow)?;
        config.total_usd_raised_cents = config
            .total_usd_raised_cents
            .checked_add(usd_cents)
            .ok_or(ErrorCode::Overflow)?;

        // Advance stage if current stage is full
        while config.stage_tokens_sold >= TOKENS_PER_STAGE * TOKEN_DECIMALS && config.current_stage < 9 {
            config.current_stage += 1;
            config.stage_tokens_sold = config.stage_tokens_sold
                .checked_sub(TOKENS_PER_STAGE * TOKEN_DECIMALS)
                .ok_or(ErrorCode::Overflow)?;
            msg!("Advanced to stage {}", config.current_stage + 1);
        }

        // Close presale if hard cap reached
        if config.tokens_sold >= PRESALE_TOTAL_ALLOCATION * TOKEN_DECIMALS {
            config.presale_active = false;
            msg!("Presale hard cap reached - presale closed");
        }

        // === CEI PATTERN: INTERACTIONS LAST ===
        // Transfer SOL payment to treasury (AFTER all state updates)
        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.sol_treasury.key(),
            sol_amount,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.sol_treasury.to_account_info(),
            ],
        )?;

        msg!(
            "Vesting stake updated: {} tokens added to {:?} tier, total staked: {}, locked until TGE ({})",
            tokens_to_stake,
            tier,
            stake_account.amount,
            config.tge_timestamp
        );

        Ok(())
    }

    // =====================================================
    // ENHANCED STAKING SYSTEM (Phase 1)
    // =====================================================

    pub fn stake_tokens(
        ctx: Context<StakeTokens>,
        amount: u64,
        tier: StakeTier,
        auto_compound: bool,
    ) -> Result<()> {
        require!(
            !ctx.accounts.user_account.is_blocked,
            ErrorCode::AddressBlocked
        );
        require!(
            amount >= ctx.accounts.config.min_stake_amount,
            ErrorCode::BelowMinimumStake
        );

        let config = &mut ctx.accounts.config;

        // Check global staking cap
        require!(
            config.total_staked.checked_add(amount).ok_or(ErrorCode::Overflow)? <= MAX_TOTAL_STAKED,
            ErrorCode::StakingCapReached
        );

        // Validate tier-specific constraints
        let lock_period_days = match tier {
            StakeTier::TierA => {
                require!(
                    config.total_staked_tier_a.checked_add(amount).ok_or(ErrorCode::Overflow)? <= MAX_STAKE_TIER_A,
                    ErrorCode::TierAFull
                );
                LOCK_PERIOD_TIER_A
            }
            StakeTier::TierB => LOCK_PERIOD_TIER_B,
            StakeTier::TierC => LOCK_PERIOD_TIER_C,
        };

        // Transfer tokens from user to program vault
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.ico_ata_for_user.to_account_info(),
                to: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, amount)?;

        let clock = Clock::get()?;
        let stake_id = config.next_stake_id;

        // Create new stake
        let stake_account = &mut ctx.accounts.stake_account;
        stake_account.stake_id = stake_id;
        stake_account.owner = ctx.accounts.user.key();
        stake_account.amount = amount;
        stake_account.start_time = clock.unix_timestamp;
        stake_account.lock_period_days = lock_period_days;
        stake_account.last_reward_calculation = clock.unix_timestamp;
        stake_account.pending_rewards = 0;
        stake_account.active = true;
        stake_account.tier = tier;
        stake_account.auto_compound = auto_compound;
        stake_account.cooldown_start = 0;
        stake_account.is_vesting = false; // Regular stake, not vesting

        // Update config
        config.next_stake_id = stake_id.checked_add(1).ok_or(ErrorCode::Overflow)?;
        config.total_staked = config
            .total_staked
            .checked_add(amount)
            .ok_or(ErrorCode::Overflow)?;

        if tier == StakeTier::TierA {
            config.total_staked_tier_a = config
                .total_staked_tier_a
                .checked_add(amount)
                .ok_or(ErrorCode::Overflow)?;
        }

        if !ctx.accounts.user_account.has_staked {
            ctx.accounts.user_account.has_staked = true;
            config.total_stakers = config
                .total_stakers
                .checked_add(1)
                .ok_or(ErrorCode::Overflow)?;
        }

        msg!(
            "Staked {} tokens in {:?} for {} days, auto_compound: {}",
            amount,
            tier,
            lock_period_days,
            auto_compound
        );
        Ok(())
    }

    /// Toggle auto-compound setting for a stake
    pub fn toggle_auto_compound(ctx: Context<ToggleAutoCompound>) -> Result<()> {
        require!(
            ctx.accounts.stake_account.owner == ctx.accounts.user.key(),
            ErrorCode::NotStakeOwner
        );
        require!(ctx.accounts.stake_account.active, ErrorCode::StakeNotActive);

        let stake_account = &mut ctx.accounts.stake_account;
        stake_account.auto_compound = !stake_account.auto_compound;

        msg!(
            "Auto-compound toggled to {} for stake {}",
            stake_account.auto_compound,
            stake_account.stake_id
        );

        Ok(())
    }

    pub fn harvest_rewards(ctx: Context<HarvestRewards>) -> Result<()> {
        require!(
            !ctx.accounts.user_account.is_blocked,
            ErrorCode::AddressBlocked
        );
        require!(
            ctx.accounts.stake_account.owner == ctx.accounts.user.key(),
            ErrorCode::NotStakeOwner
        );
        require!(ctx.accounts.stake_account.active, ErrorCode::StakeNotActive);

        let clock = Clock::get()?;
        let rewards = calculate_rewards_internal(
            &ctx.accounts.config,
            &ctx.accounts.stake_account,
            clock.unix_timestamp,
        )?;

        require!(rewards > 0, ErrorCode::NoRewards);

        let stake_account = &mut ctx.accounts.stake_account;

        // Handle auto-compound
        if stake_account.auto_compound {
            // === CEI: EFFECTS ===
            // Add rewards to principal
            stake_account.amount = stake_account
                .amount
                .checked_add(rewards)
                .ok_or(ErrorCode::Overflow)?;
            stake_account.pending_rewards = 0;
            stake_account.last_reward_calculation = clock.unix_timestamp;

            msg!("Auto-compounded {} rewards into stake", rewards);
        } else {
            // === CEI: EFFECTS FIRST (state changes before transfer) ===
            stake_account.pending_rewards = 0;
            stake_account.last_reward_calculation = clock.unix_timestamp;
            ctx.accounts.config.total_rewards_distributed = ctx
                .accounts
                .config
                .total_rewards_distributed
                .checked_add(rewards)
                .ok_or(ErrorCode::Overflow)?;

            // === CEI: INTERACTIONS LAST ===
            // Transfer rewards to user
            let ico_mint_key = ctx.accounts.ico_mint.key();
            let (expected_pda, bump) = Pubkey::find_program_address(
                &[ico_mint_key.as_ref()],
                ctx.program_id,
            );
            
            require!(
                ctx.accounts.ico_ata_for_ico_program.key() == expected_pda,
                ErrorCode::InvalidPDA
            );
            
            let seeds = &[ico_mint_key.as_ref(), &[bump]];
            let signer = &[&seeds[..]];

            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
                    to: ctx.accounts.ico_ata_for_user.to_account_info(),
                    authority: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
                },
                signer,
            );
            token::transfer(cpi_ctx, rewards)?;

            msg!("Harvested {} rewards", rewards);
        }

        Ok(())
    }

    /// Initiate unstake - starts 48-hour cooldown
    /// Vesting stakes can only be unstaked after TGE
    pub fn initiate_unstake(ctx: Context<InitiateUnstake>) -> Result<()> {
        require!(
            !ctx.accounts.user_account.is_blocked,
            ErrorCode::AddressBlocked
        );
        require!(
            ctx.accounts.stake_account.owner == ctx.accounts.user.key(),
            ErrorCode::NotStakeOwner
        );
        require!(ctx.accounts.stake_account.active, ErrorCode::StakeNotActive);

        let clock = Clock::get()?;
        let stake_account = &ctx.accounts.stake_account;
        let config = &ctx.accounts.config;

        // VESTING CHECK: If this is a vesting stake, check if TGE has passed
        if stake_account.is_vesting {
            require!(
                clock.unix_timestamp >= config.tge_timestamp,
                ErrorCode::VestingStakeLocked
            );
            msg!("Vesting stake - TGE has passed, unstake allowed");
        }

        // Check if lock period has passed
        let unlock_time = stake_account
            .start_time
            .checked_add(
                (stake_account.lock_period_days as i64)
                    .checked_mul(SECONDS_PER_DAY)
                    .ok_or(ErrorCode::Overflow)?,
            )
            .ok_or(ErrorCode::Overflow)?;

        require!(
            clock.unix_timestamp >= unlock_time,
            ErrorCode::StillInLockPeriod
        );

        let stake_account = &mut ctx.accounts.stake_account;
        stake_account.cooldown_start = clock.unix_timestamp;

        msg!(
            "Unstake initiated for stake {}. Can finalize after 48 hours.",
            stake_account.stake_id
        );

        Ok(())
    }

    /// Finalize unstake after 48-hour cooldown
    pub fn finalize_unstake(ctx: Context<FinalizeUnstake>) -> Result<()> {
        require!(
            !ctx.accounts.user_account.is_blocked,
            ErrorCode::AddressBlocked
        );
        require!(
            ctx.accounts.stake_account.owner == ctx.accounts.user.key(),
            ErrorCode::NotStakeOwner
        );
        require!(ctx.accounts.stake_account.active, ErrorCode::StakeNotActive);

        let clock = Clock::get()?;
        let stake_account = &ctx.accounts.stake_account;

        // Validate cooldown period has passed
        require!(stake_account.cooldown_start > 0, ErrorCode::InvalidAmount);
        let cooldown_end = stake_account
            .cooldown_start
            .checked_add(COOLDOWN_PERIOD)
            .ok_or(ErrorCode::Overflow)?;
        require!(
            clock.unix_timestamp >= cooldown_end,
            ErrorCode::StillInCooldown
        );

        // Calculate final rewards
        let rewards = calculate_rewards_internal(
            &ctx.accounts.config,
            stake_account,
            clock.unix_timestamp,
        )?;

        let stake_amount = stake_account.amount;
        let tier = stake_account.tier;
        let total_amount = stake_amount
            .checked_add(rewards)
            .ok_or(ErrorCode::Overflow)?;

        // Deactivate stake
        let stake_account = &mut ctx.accounts.stake_account;
        stake_account.active = false;

        // Update config
        let config = &mut ctx.accounts.config;
        config.total_staked = config
            .total_staked
            .checked_sub(stake_amount)
            .ok_or(ErrorCode::Overflow)?;

        if tier == StakeTier::TierA {
            config.total_staked_tier_a = config
                .total_staked_tier_a
                .checked_sub(stake_amount)
                .ok_or(ErrorCode::Overflow)?;
        }

        if rewards > 0 {
            config.total_rewards_distributed = config
                .total_rewards_distributed
                .checked_add(rewards)
                .ok_or(ErrorCode::Overflow)?;
        }

        // Transfer principal + rewards to user
        let ico_mint_key = ctx.accounts.ico_mint.key();
        let (expected_pda, bump) = Pubkey::find_program_address(
            &[ico_mint_key.as_ref()],
            ctx.program_id,
        );
        
        require!(
            ctx.accounts.ico_ata_for_ico_program.key() == expected_pda,
            ErrorCode::InvalidPDA
        );
        
        let seeds = &[ico_mint_key.as_ref(), &[bump]];
        let signer = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
                to: ctx.accounts.ico_ata_for_user.to_account_info(),
                authority: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
            },
            signer,
        );
        token::transfer(cpi_ctx, total_amount)?;

        msg!(
            "Unstaked {} tokens with {} rewards (total: {})",
            stake_amount,
            rewards,
            total_amount
        );

        Ok(())
    }

    // =====================================================
    // ADMIN FUNCTIONS
    // =====================================================

    pub fn update_tge_timestamp(ctx: Context<UpdateConfig>, new_tge_timestamp: i64) -> Result<()> {
        require!(
            ctx.accounts.config.admin == ctx.accounts.admin.key(),
            ErrorCode::InvalidAdmin
        );

        ctx.accounts.config.tge_timestamp = new_tge_timestamp;
        msg!("Updated TGE timestamp to {}", new_tge_timestamp);
        Ok(())
    }

    pub fn set_presale_active(ctx: Context<UpdateConfig>, active: bool) -> Result<()> {
        require!(
            ctx.accounts.config.admin == ctx.accounts.admin.key(),
            ErrorCode::InvalidAdmin
        );

        ctx.accounts.config.presale_active = active;
        msg!("Presale active set to {}", active);
        Ok(())
    }

    /// Resize config account to accommodate new fields
    /// This is an admin-only migration function
    pub fn resize_config(ctx: Context<ResizeConfig>) -> Result<()> {
        // Read admin pubkey from raw account data (after 8-byte discriminator)
        let config_data = ctx.accounts.config.try_borrow_data()?;
        let admin_bytes: [u8; 32] = config_data[8..40].try_into().unwrap();
        let stored_admin = Pubkey::from(admin_bytes);
        drop(config_data);
        
        require!(
            stored_admin == ctx.accounts.admin.key(),
            ErrorCode::InvalidAdmin
        );
        
        let new_size = 8 + Config::SPACE; // discriminator + data
        let rent = Rent::get()?;
        let new_minimum_balance = rent.minimum_balance(new_size);
        
        let current_balance = ctx.accounts.config.lamports();
        let lamports_diff = new_minimum_balance.saturating_sub(current_balance);
        
        if lamports_diff > 0 {
            // Transfer lamports from admin to config account
            let cpi_context = CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.admin.to_account_info(),
                    to: ctx.accounts.config.clone(),
                },
            );
            anchor_lang::system_program::transfer(cpi_context, lamports_diff)?;
        }
        
        // Reallocate the account using new resize() method
        ctx.accounts.config.resize(new_size)?;
        
        msg!("Config account resized to {} bytes", new_size);
        Ok(())
    }

    pub fn update_token_price(ctx: Context<UpdateConfig>, new_price: u64) -> Result<()> {
        require!(
            ctx.accounts.config.admin == ctx.accounts.admin.key(),
            ErrorCode::InvalidAdmin
        );
        require!(new_price > 0, ErrorCode::InvalidPrice);

        ctx.accounts.config.sol_price_for_token = new_price;
        msg!("Updated token price to {}", new_price);
        Ok(())
    }

    /// Admin function to update presale start time
    pub fn update_presale_start_time(ctx: Context<UpdateConfig>, new_start_time: i64) -> Result<()> {
        require!(
            ctx.accounts.config.admin == ctx.accounts.admin.key(),
            ErrorCode::InvalidAdmin
        );

        ctx.accounts.config.presale_start_time = new_start_time;
        msg!("Updated presale start time to {}", new_start_time);
        Ok(())
    }

    pub fn set_block_status(ctx: Context<SetBlockStatus>, blocked: bool) -> Result<()> {
        require!(
            ctx.accounts.config.admin == ctx.accounts.admin.key(),
            ErrorCode::InvalidAdmin
        );

        ctx.accounts.user_account.is_blocked = blocked;
        msg!("Updated block status for user");
        Ok(())
    }

    pub fn set_stablecoin_addresses(
        ctx: Context<UpdateConfig>,
        usdt_address: Pubkey,
        usdc_address: Pubkey,
    ) -> Result<()> {
        require!(
            ctx.accounts.config.admin == ctx.accounts.admin.key(),
            ErrorCode::InvalidAdmin
        );

        ctx.accounts.config.usdt_address = usdt_address;
        ctx.accounts.config.usdc_address = usdc_address;

        msg!("Updated stablecoin addresses - USDT: {}, USDC: {}", usdt_address, usdc_address);
        Ok(())
    }

    pub fn withdraw_tokens(ctx: Context<WithdrawTokens>, amount: u64) -> Result<()> {
        require!(
            ctx.accounts.config.admin == ctx.accounts.admin.key(),
            ErrorCode::InvalidAdmin
        );

        // Ensure we're not withdrawing staked tokens
        let available_balance = ctx
            .accounts
            .ico_ata_for_ico_program
            .amount
            .checked_sub(ctx.accounts.config.total_staked)
            .ok_or(ErrorCode::Overflow)?;

        require!(
            amount <= available_balance,
            ErrorCode::InsufficientBalance
        );

        let ico_mint_key = ctx.accounts.ico_mint.key();
        let (expected_pda, bump) = Pubkey::find_program_address(
            &[ico_mint_key.as_ref()],
            ctx.program_id,
        );
        
        require!(
            ctx.accounts.ico_ata_for_ico_program.key() == expected_pda,
            ErrorCode::InvalidPDA
        );
        
        let seeds = &[ico_mint_key.as_ref(), &[bump]];
        let signer = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
                to: ctx.accounts.ico_ata_for_admin.to_account_info(),
                authority: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
            },
            signer,
        );
        token::transfer(cpi_ctx, amount)?;

        msg!("Withdrew {} tokens", amount);
        Ok(())
    }

    // =====================================================
    // TEAM VESTING FUNCTIONS (Admin Only)
    // =====================================================

    /// Create team vesting allocation for a team member (admin only)
    /// Tokens are locked for 18 months after TGE
    pub fn create_team_vesting(
        ctx: Context<CreateTeamVesting>,
        allocation_amount: u64,
    ) -> Result<()> {
        let config = &ctx.accounts.config;
        let clock = Clock::get()?;

        // Validate admin
        require!(
            config.admin == ctx.accounts.admin.key(),
            ErrorCode::InvalidAdmin
        );

        // Calculate raw amount with decimals
        let raw_amount = allocation_amount
            .checked_mul(TOKEN_DECIMALS)
            .ok_or(ErrorCode::Overflow)?;

        // Validate allocation doesn't exceed team allocation limit
        require!(
            raw_amount <= TEAM_ALLOCATION * TOKEN_DECIMALS,
            ErrorCode::ExceedsTeamAllocation
        );

        // Calculate cliff end (TGE + 18 months)
        let cliff_end = config.tge_timestamp
            .checked_add(TEAM_LOCK_PERIOD_SECONDS)
            .ok_or(ErrorCode::Overflow)?;

        // Initialize team vesting account
        let team_vesting = &mut ctx.accounts.team_vesting;
        team_vesting.member = ctx.accounts.team_member.key();
        team_vesting.total_allocation = raw_amount;
        team_vesting.claimed_amount = 0;
        team_vesting.created_at = clock.unix_timestamp;
        team_vesting.cliff_end = cliff_end;
        team_vesting.is_active = true;

        msg!(
            "Created team vesting for {}: {} tokens, cliff ends at {}",
            ctx.accounts.team_member.key(),
            allocation_amount,
            cliff_end
        );

        Ok(())
    }

    /// Claim team tokens after 18-month lockup period (team member only)
    pub fn claim_team_tokens(ctx: Context<ClaimTeamTokens>) -> Result<()> {
        let _config = &ctx.accounts.config;
        let clock = Clock::get()?;
        let team_vesting = &mut ctx.accounts.team_vesting;

        // === CHECKS ===
        // Validate team member is the owner
        require!(
            team_vesting.member == ctx.accounts.team_member.key(),
            ErrorCode::InvalidAdmin
        );

        // Validate vesting is active
        require!(team_vesting.is_active, ErrorCode::TeamAllocationNotFound);

        // Check if 18-month lockup has passed
        require!(
            clock.unix_timestamp >= team_vesting.cliff_end,
            ErrorCode::TeamTokensLocked
        );

        // Calculate claimable amount
        let claimable = team_vesting.total_allocation
            .checked_sub(team_vesting.claimed_amount)
            .ok_or(ErrorCode::Overflow)?;

        require!(claimable > 0, ErrorCode::NoTeamTokensToClaim);

        // === CEI: EFFECTS FIRST (state changes before transfer) ===
        team_vesting.claimed_amount = team_vesting.total_allocation;
        team_vesting.is_active = false;

        // === CEI: INTERACTIONS LAST ===
        // Transfer tokens from program ATA to team member
        let ico_mint_key = ctx.accounts.ico_mint.key();
        let (_, bump) = Pubkey::find_program_address(
            &[ico_mint_key.as_ref()],
            ctx.program_id,
        );
        
        let seeds = &[ico_mint_key.as_ref(), &[bump]];
        let signer = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
                to: ctx.accounts.team_member_ata.to_account_info(),
                authority: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
            },
            signer,
        );
        token::transfer(cpi_ctx, claimable)?;

        msg!(
            "Team member {} claimed {} tokens",
            ctx.accounts.team_member.key(),
            claimable
        );

        Ok(())
    }

    /// View function to check team vesting status (anyone can call)
    pub fn get_team_vesting_status(ctx: Context<GetTeamVestingStatus>) -> Result<()> {
        let team_vesting = &ctx.accounts.team_vesting;
        let _config = &ctx.accounts.config;
        let clock = Clock::get()?;

        let time_until_unlock = team_vesting.cliff_end
            .saturating_sub(clock.unix_timestamp);
        
        let is_unlocked = clock.unix_timestamp >= team_vesting.cliff_end;
        let claimable = if is_unlocked {
            team_vesting.total_allocation.saturating_sub(team_vesting.claimed_amount)
        } else {
            0
        };

        msg!(
            "Team Vesting Status - Member: {}, Total: {}, Claimed: {}, Claimable: {}, Unlocked: {}, Days until unlock: {}",
            team_vesting.member,
            team_vesting.total_allocation,
            team_vesting.claimed_amount,
            claimable,
            is_unlocked,
            time_until_unlock / 86400
        );

        Ok(())
    }

    // =====================================================
    // CROSS-CHAIN FUNCTIONS (Coordinator Only)
    // =====================================================

    /// Set the cross-chain coordinator address (admin only)
    pub fn set_coordinator(ctx: Context<UpdateConfig>, coordinator: Pubkey) -> Result<()> {
        require!(
            ctx.accounts.config.admin == ctx.accounts.admin.key(),
            ErrorCode::InvalidAdmin
        );

        ctx.accounts.config.coordinator = coordinator;
        msg!("Coordinator set to {}", coordinator);
        Ok(())
    }

    /// Admin function to update maximum purchase limit per user
    /// Set to 0 to use the default constant PRESALE_MAX_PURCHASE_USD
    pub fn update_max_per_user(ctx: Context<UpdateConfig>, max_usd_cents: u64) -> Result<()> {
        require!(
            ctx.accounts.config.admin == ctx.accounts.admin.key(),
            ErrorCode::InvalidAdmin
        );

        ctx.accounts.config.max_per_user_usd = max_usd_cents;
        msg!("Updated max per user to {} cents", max_usd_cents);
        Ok(())
    }

    /// Admin function to update minimum purchase limit
    /// Set to 0 to use the default constant PRESALE_MIN_PURCHASE_USD
    pub fn update_min_purchase(ctx: Context<UpdateConfig>, min_usd_cents: u64) -> Result<()> {
        require!(
            ctx.accounts.config.admin == ctx.accounts.admin.key(),
            ErrorCode::InvalidAdmin
        );

        ctx.accounts.config.min_purchase_usd = min_usd_cents;
        msg!("Updated min purchase to {} cents", min_usd_cents);
        Ok(())
    }

    /// Admin function to update SOL treasury address (for Squads multisig)
    /// SOL payments will be sent to this address instead of admin
    pub fn update_sol_treasury(ctx: Context<UpdateConfig>, new_treasury: Pubkey) -> Result<()> {
        require!(
            ctx.accounts.config.admin == ctx.accounts.admin.key(),
            ErrorCode::InvalidAdmin
        );

        let old_treasury = ctx.accounts.config.sol_treasury;
        ctx.accounts.config.sol_treasury = new_treasury;
        msg!("Updated SOL treasury from {} to {}", old_treasury, new_treasury);
        Ok(())
    }

    /// Record a cross-chain purchase from ETH/BNB (coordinator only)
    /// This function is called by the coordinator when a purchase is made on EVM chains
    pub fn record_cross_chain_purchase(
        ctx: Context<RecordCrossChainPurchase>,
        buyer_eth_address: [u8; 20],
        chain_id: u8,
        noc_amount: u64,
        usd_cents: u64,
        _tx_hash: [u8; 32],
        _stage: u8,
        referrer_eth: [u8; 20],
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        let clock = Clock::get()?;

        // Validate coordinator
        require!(
            ctx.accounts.coordinator.key() == config.coordinator,
            ErrorCode::InvalidCoordinator
        );

        // Validate presale is active
        require!(config.presale_active, ErrorCode::PresaleNotStarted);
        require!(
            config.tokens_sold.checked_add(noc_amount).ok_or(ErrorCode::Overflow)? 
                <= PRESALE_TOTAL_ALLOCATION * TOKEN_DECIMALS,
            ErrorCode::PresaleHardCapReached
        );

        // Validate chain ID (1=ETH, 56=BNB, 137=Polygon)
        require!(
            chain_id == 1 || chain_id == 56 || chain_id == 137,
            ErrorCode::InvalidChainId
        );

        // === CROSS-CHAIN SECURITY CHECKS ===
        
        // 1. Validate minimum purchase ($10)
        require!(
            usd_cents >= CROSS_CHAIN_MIN_USD_CENTS,
            ErrorCode::CrossChainBelowMinimum
        );
        
        // 2. Validate maximum per transaction ($10,000)
        require!(
            usd_cents <= CROSS_CHAIN_MAX_USD_CENTS,
            ErrorCode::CrossChainExceedsMaximum
        );

        let allocation = &mut ctx.accounts.cross_chain_allocation;

        // 3. Check cooldown (30 seconds) - only for existing allocations
        if allocation.chain_id != 0 {
            let time_since_last = clock.unix_timestamp
                .checked_sub(allocation.last_purchase_at)
                .ok_or(ErrorCode::Overflow)?;
            require!(
                time_since_last >= CROSS_CHAIN_COOLDOWN_SECONDS,
                ErrorCode::CrossChainCooldown
            );
        }
        
        // 4. Check user total limit ($50,000)
        let new_total_usd = if allocation.chain_id == 0 {
            usd_cents
        } else {
            allocation.total_usd_cents
                .checked_add(usd_cents)
                .ok_or(ErrorCode::Overflow)?
        };
        require!(
            new_total_usd <= CROSS_CHAIN_MAX_TOTAL_USD_CENTS,
            ErrorCode::CrossChainUserLimitExceeded
        );

        // Initialize or update allocation
        if allocation.chain_id == 0 {
            // First purchase
            allocation.eth_address = buyer_eth_address;
            allocation.chain_id = chain_id;
            allocation.total_tokens = noc_amount;
            allocation.total_usd_cents = usd_cents;
            allocation.purchase_count = 1;
            allocation.first_purchase_at = clock.unix_timestamp;
            allocation.last_purchase_at = clock.unix_timestamp;
            allocation.referrer_eth = referrer_eth;
            allocation.linked_solana_wallet = Pubkey::default();
            allocation.claimed = false;
        } else {
            // Additional purchase
            allocation.total_tokens = allocation
                .total_tokens
                .checked_add(noc_amount)
                .ok_or(ErrorCode::Overflow)?;
            allocation.total_usd_cents = allocation
                .total_usd_cents
                .checked_add(usd_cents)
                .ok_or(ErrorCode::Overflow)?;
            allocation.purchase_count = allocation
                .purchase_count
                .checked_add(1)
                .ok_or(ErrorCode::Overflow)?;
            allocation.last_purchase_at = clock.unix_timestamp;
        }

        // Process referral bonus (10%) - ONE-TIME only on first purchase
        let referral_bonus = if referrer_eth != [0u8; 20] && allocation.purchase_count == 1 {
            let bonus = noc_amount
                .checked_mul(REFERRAL_BONUS_PERCENTAGE)
                .ok_or(ErrorCode::Overflow)?
                .checked_div(100)
                .ok_or(ErrorCode::Overflow)?;

            // Check Community Rewards pool
            let new_total_referral = config.total_referral_bonuses
                .checked_add(bonus)
                .ok_or(ErrorCode::Overflow)?;

            if new_total_referral <= COMMUNITY_REWARDS_ALLOCATION * TOKEN_DECIMALS {
                allocation.referral_bonus = bonus;
                config.total_referral_bonuses = new_total_referral;
                
                // Update referrer's allocation if it exists
                if !ctx.accounts.referrer_cross_chain_allocation.data_is_empty() {
                    let mut referrer_data = ctx.accounts.referrer_cross_chain_allocation.try_borrow_mut_data()?;
                    if referrer_data.len() >= 8 + CrossChainAllocation::SPACE {
                        // Add bonus to referrer
                        let mut referrer_alloc = CrossChainAllocation::try_deserialize(&mut &referrer_data[..])?;
                        referrer_alloc.total_tokens = referrer_alloc
                            .total_tokens
                            .checked_add(bonus)
                            .ok_or(ErrorCode::Overflow)?;
                        referrer_alloc.try_serialize(&mut &mut referrer_data[..])?;
                    }
                }
                
                msg!("One-time referral bonus: {} tokens", bonus);
                bonus
            } else {
                msg!("Referral pool exhausted");
                0
            }
        } else {
            0
        };

        // Update global stats
        config.tokens_sold = config
            .tokens_sold
            .checked_add(noc_amount)
            .ok_or(ErrorCode::Overflow)?;
        config.cross_chain_tokens_sold = config
            .cross_chain_tokens_sold
            .checked_add(noc_amount)
            .ok_or(ErrorCode::Overflow)?;
        config.total_usd_raised_cents = config
            .total_usd_raised_cents
            .checked_add(usd_cents)
            .ok_or(ErrorCode::Overflow)?;

        // Update stage progress
        config.stage_tokens_sold = config
            .stage_tokens_sold
            .checked_add(noc_amount)
            .ok_or(ErrorCode::Overflow)?;

        // Advance stage if full
        while config.stage_tokens_sold >= TOKENS_PER_STAGE * TOKEN_DECIMALS && config.current_stage < 9 {
            config.current_stage += 1;
            config.stage_tokens_sold = config.stage_tokens_sold
                .checked_sub(TOKENS_PER_STAGE * TOKEN_DECIMALS)
                .ok_or(ErrorCode::Overflow)?;
            msg!("Stage advanced to {}", config.current_stage + 1);
        }

        msg!(
            "Cross-chain purchase recorded: chain={}, buyer={:?}, noc={}, usd_cents={}, referral_bonus={}",
            chain_id,
            &buyer_eth_address[..4],
            noc_amount,
            usd_cents,
            referral_bonus
        );

        Ok(())
    }

    /// Link a Solana wallet to a cross-chain allocation (user signs with ETH signature verified off-chain)
    pub fn link_solana_wallet(
        ctx: Context<LinkSolanaWallet>,
        eth_address: [u8; 20],
        chain_id: u8,
    ) -> Result<()> {
        let allocation = &mut ctx.accounts.cross_chain_allocation;
        
        // Verify the allocation belongs to this ETH address
        require!(
            allocation.eth_address == eth_address && allocation.chain_id == chain_id,
            ErrorCode::InvalidAddress
        );
        
        // Link the Solana wallet
        allocation.linked_solana_wallet = ctx.accounts.user.key();
        
        msg!(
            "Linked Solana wallet {} to ETH address {:?}",
            ctx.accounts.user.key(),
            &eth_address[..4]
        );
        
        Ok(())
    }

    /// Claim cross-chain allocation at TGE
    pub fn claim_cross_chain_allocation(ctx: Context<ClaimCrossChainAllocation>) -> Result<()> {
        let config = &ctx.accounts.config;
        let clock = Clock::get()?;
        let allocation = &mut ctx.accounts.cross_chain_allocation;

        // === CHECKS ===
        // Validate TGE has occurred
        require!(
            clock.unix_timestamp >= config.tge_timestamp,
            ErrorCode::TokensLocked
        );

        // Validate allocation exists and not claimed
        require!(allocation.total_tokens > 0, ErrorCode::InvalidAmount);
        require!(!allocation.claimed, ErrorCode::AllocationAlreadyClaimed);

        // Validate user is the linked wallet
        require!(
            allocation.linked_solana_wallet == ctx.accounts.user.key(),
            ErrorCode::NotStakeOwner
        );

        let total_to_claim = allocation.total_tokens;

        // === CEI: EFFECTS FIRST (state changes before transfer) ===
        allocation.claimed = true;

        // === CEI: INTERACTIONS LAST ===
        // Transfer tokens
        let ico_mint_key = ctx.accounts.ico_mint.key();
        let (expected_pda, bump) = Pubkey::find_program_address(
            &[ico_mint_key.as_ref()],
            ctx.program_id,
        );
        
        require!(
            ctx.accounts.ico_ata_for_ico_program.key() == expected_pda,
            ErrorCode::InvalidPDA
        );
        
        let seeds = &[ico_mint_key.as_ref(), &[bump]];
        let signer = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
                to: ctx.accounts.ico_ata_for_user.to_account_info(),
                authority: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
            },
            signer,
        );
        token::transfer(cpi_ctx, total_to_claim)?;

        msg!(
            "Cross-chain claim: {} tokens to {}",
            total_to_claim,
            ctx.accounts.user.key()
        );

        Ok(())
    }

    /// Coordinator-initiated mint and vesting stake for EVM buyers
    /// This allows EVM buyers to receive tokens immediately (minted + staked) instead of waiting for TGE
    /// Called by coordinator when EVM buyer provides a Solana address
    pub fn coordinator_mint_and_vest_stake(
        ctx: Context<CoordinatorMintAndVestStake>,
        buyer_eth_address: [u8; 20],
        chain_id: u8,
        noc_amount: u64,
        usd_cents: u64,
        tier: StakeTier,
        auto_compound: bool,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        let clock = Clock::get()?;

        // Validate coordinator
        require!(
            ctx.accounts.coordinator.key() == config.coordinator,
            ErrorCode::InvalidCoordinator
        );

        // Validate presale is active
        require!(config.presale_active, ErrorCode::PresaleNotStarted);
        require!(
            config.tokens_sold.checked_add(noc_amount).ok_or(ErrorCode::Overflow)? 
                <= PRESALE_TOTAL_ALLOCATION * TOKEN_DECIMALS,
            ErrorCode::PresaleHardCapReached
        );

        // Get lock period based on tier (using existing tier definitions)
        // TierA = 365 days, TierB = 182 days, TierC = 90 days
        let lock_period_days = match tier {
            StakeTier::TierA => 365,
            StakeTier::TierB => 182,
            StakeTier::TierC => 90,
        };

        // === CEI: EFFECTS FIRST (all state changes before transfer) ===
        
        // Initialize stake account
        let stake_account = &mut ctx.accounts.stake_account;
        stake_account.stake_id = config.next_stake_id;
        stake_account.owner = ctx.accounts.beneficiary.key();
        stake_account.amount = noc_amount;
        stake_account.start_time = clock.unix_timestamp;
        stake_account.lock_period_days = lock_period_days;
        stake_account.last_reward_calculation = clock.unix_timestamp;
        stake_account.pending_rewards = 0;
        stake_account.active = true;
        stake_account.tier = tier;
        stake_account.auto_compound = auto_compound;
        stake_account.cooldown_start = 0;
        stake_account.is_vesting = true; // Mark as vesting stake - locked until TGE

        // Update cross-chain allocation to track this
        let allocation = &mut ctx.accounts.cross_chain_allocation;
        if allocation.chain_id == 0 {
            // First purchase - initialize
            allocation.eth_address = buyer_eth_address;
            allocation.chain_id = chain_id;
            allocation.total_tokens = noc_amount;
            allocation.total_usd_cents = usd_cents;
            allocation.purchase_count = 1;
            allocation.first_purchase_at = clock.unix_timestamp;
            allocation.last_purchase_at = clock.unix_timestamp;
            allocation.linked_solana_wallet = ctx.accounts.beneficiary.key();
            allocation.claimed = true; // Mark as claimed since we're minting directly
        } else {
            // Additional purchase
            allocation.total_tokens = allocation
                .total_tokens
                .checked_add(noc_amount)
                .ok_or(ErrorCode::Overflow)?;
            allocation.total_usd_cents = allocation
                .total_usd_cents
                .checked_add(usd_cents)
                .ok_or(ErrorCode::Overflow)?;
            allocation.purchase_count = allocation
                .purchase_count
                .checked_add(1)
                .ok_or(ErrorCode::Overflow)?;
            allocation.last_purchase_at = clock.unix_timestamp;
        }

        // Update global stats
        config.tokens_sold = config
            .tokens_sold
            .checked_add(noc_amount)
            .ok_or(ErrorCode::Overflow)?;
        config.cross_chain_tokens_sold = config
            .cross_chain_tokens_sold
            .checked_add(noc_amount)
            .ok_or(ErrorCode::Overflow)?;
        config.total_usd_raised_cents = config
            .total_usd_raised_cents
            .checked_add(usd_cents)
            .ok_or(ErrorCode::Overflow)?;

        // Update staking stats
        config.total_staked = config
            .total_staked
            .checked_add(noc_amount)
            .ok_or(ErrorCode::Overflow)?;
        config.next_stake_id = config
            .next_stake_id
            .checked_add(1)
            .ok_or(ErrorCode::Overflow)?;
        config.total_stakers = config
            .total_stakers
            .checked_add(1)
            .ok_or(ErrorCode::Overflow)?;

        // Update stage progress
        config.stage_tokens_sold = config
            .stage_tokens_sold
            .checked_add(noc_amount)
            .ok_or(ErrorCode::Overflow)?;

        // Advance stage if full
        while config.stage_tokens_sold >= TOKENS_PER_STAGE * TOKEN_DECIMALS && config.current_stage < 9 {
            config.current_stage += 1;
            config.stage_tokens_sold = config.stage_tokens_sold
                .checked_sub(TOKENS_PER_STAGE * TOKEN_DECIMALS)
                .ok_or(ErrorCode::Overflow)?;
            msg!("Stage advanced to {}", config.current_stage + 1);
        }

        // === CEI: INTERACTIONS LAST ===
        // Transfer tokens from program treasury to stake pool
        // The ico_ata_for_ico_program is a PDA token account with self-authority
        let ico_mint_key = ctx.accounts.ico_mint.key();
        let (_, treasury_bump) = Pubkey::find_program_address(
            &[ico_mint_key.as_ref()],
            ctx.program_id,
        );
        let treasury_seeds = &[ico_mint_key.as_ref(), &[treasury_bump]];
        let treasury_signer = &[&treasury_seeds[..]];

        // Transfer tokens from treasury to stake pool
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
                to: ctx.accounts.stake_pool_ata.to_account_info(),
                authority: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
            },
            treasury_signer,
        );
        token::transfer(cpi_ctx, noc_amount)?;

        msg!(
            "Coordinator mint & vest stake: chain={}, eth={:?}, solana={}, noc={}, tier={:?}",
            chain_id,
            &buyer_eth_address[..4],
            ctx.accounts.beneficiary.key(),
            noc_amount,
            tier
        );

        Ok(())
    }
}

// =====================================================
// HELPER FUNCTIONS
// =====================================================

/// Internal helper for stablecoin purchases (USDT/USDC)
fn presale_purchase_with_stablecoin_internal<'info>(
    config: &mut Account<'info, Config>,
    user_account: &mut Account<'info, UserAccount>,
    user_allocation: &mut Account<'info, PresaleAllocation>,
    referrer_allocation: &AccountInfo<'info>,
    stablecoin_ata_for_user: &Account<'info, TokenAccount>,
    stablecoin_ata_for_admin: &Account<'info, TokenAccount>,
    _stablecoin_mint: &Account<'info, Mint>,
    user: &Signer<'info>,
    token_program: &Program<'info, Token>,
    stablecoin_amount: u64,
    coin_name: &str,
) -> Result<()> {
    let clock = Clock::get()?;

    // Validate presale is active
    require!(config.presale_active, ErrorCode::PresaleNotStarted);
    require!(
        clock.unix_timestamp >= config.presale_start_time,
        ErrorCode::PresaleNotStarted
    );
    require!(
        config.tokens_sold < PRESALE_TOTAL_ALLOCATION * TOKEN_DECIMALS,
        ErrorCode::PresaleHardCapReached
    );

    // Validate user not blocked
    require!(!user_account.is_blocked, ErrorCode::AddressBlocked);
    require!(stablecoin_amount > 0, ErrorCode::InvalidAmount);

    // Convert stablecoin amount to USD cents
    // USDT/USDC have 6 decimals, so 1_000_000 = $1.00 = 100 cents
    let usd_cents = stablecoin_amount
        .checked_mul(100)
        .ok_or(ErrorCode::Overflow)?
        .checked_div(1_000_000)
        .ok_or(ErrorCode::Overflow)?;

    msg!("{} amount: {}, USD value: {} cents", coin_name, stablecoin_amount, usd_cents);

    // Validate purchase limits (use config values if set, otherwise use constants)
    let min_purchase = if config.min_purchase_usd > 0 { config.min_purchase_usd } else { PRESALE_MIN_PURCHASE_USD };
    let max_purchase = if config.max_per_user_usd > 0 { config.max_per_user_usd } else { PRESALE_MAX_PURCHASE_USD };
    
    // Check minimum per transaction
    require!(usd_cents >= min_purchase, ErrorCode::BelowMinimumPurchase);
    
    // Check maximum per transaction ($10,000)
    require!(usd_cents <= max_purchase, ErrorCode::ExceedsMaximumPurchase);
    
    let new_total_spent = user_allocation.total_spent_cents
        .checked_add(usd_cents)
        .ok_or(ErrorCode::Overflow)?;
    
    // Check maximum total per user ($25,600)
    require!(
        new_total_spent <= PRESALE_MAX_TOTAL_PER_USER_USD,
        ErrorCode::ExceedsMaximumPurchase
    );

    // Calculate tokens based on current stage price
    let tokens_to_allocate = calculate_tokens_for_usd(usd_cents, config.current_stage)?;

    // Transfer stablecoin from user to admin
    let cpi_ctx = CpiContext::new(
        token_program.to_account_info(),
        Transfer {
            from: stablecoin_ata_for_user.to_account_info(),
            to: stablecoin_ata_for_admin.to_account_info(),
            authority: user.to_account_info(),
        },
    );
    token::transfer(cpi_ctx, stablecoin_amount)?;

    // Process referral bonus (10%) - FROM COMMUNITY REWARDS POOL (not presale)
    // ONE-TIME ONLY: Referral bonus is only given on the FIRST purchase
    let referral_bonus = if user_allocation.referrer != Pubkey::default() && user_allocation.purchase_count == 0 {
        let bonus = tokens_to_allocate
            .checked_mul(REFERRAL_BONUS_PERCENTAGE)
            .ok_or(ErrorCode::Overflow)?
            .checked_div(100)
            .ok_or(ErrorCode::Overflow)?;

        // Check if Community Rewards pool has enough tokens
        let new_total_referral = config.total_referral_bonuses
            .checked_add(bonus)
            .ok_or(ErrorCode::Overflow)?;
        
        if new_total_referral > COMMUNITY_REWARDS_ALLOCATION * TOKEN_DECIMALS {
            // Pool exhausted - no bonus but purchase continues
            msg!("Referral pool exhausted - no bonus awarded");
            0
        } else if !referrer_allocation.data_is_empty() {
            // SECURITY: Validate referrer_allocation PDA matches expected seeds
            let referrer_pubkey = user_allocation.referrer;
            let (expected_pda, _bump) = Pubkey::find_program_address(
                &[b"allocation", referrer_pubkey.as_ref()],
                &crate::ID
            );
            if referrer_allocation.key() != expected_pda {
                msg!("Invalid referrer PDA - skipping bonus");
                0
            } else {
            
            let mut referrer_data = referrer_allocation.try_borrow_mut_data()?;
            let mut referrer_alloc = PresaleAllocation::try_deserialize(&mut &referrer_data[..])?;
            
            referrer_alloc.referral_bonus_tokens = referrer_alloc
                .referral_bonus_tokens
                .checked_add(bonus)
                .ok_or(ErrorCode::Overflow)?;
            referrer_alloc.total_tokens = referrer_alloc
                .total_tokens
                .checked_add(bonus)
                .ok_or(ErrorCode::Overflow)?;
            
            referrer_alloc.try_serialize(&mut &mut referrer_data[..])?;
            
            // Track total referral bonuses issued (from Community Rewards pool)
            config.total_referral_bonuses = new_total_referral;
            
            msg!("One-time referral bonus awarded: {} tokens", bonus);
            bonus
            }
        } else {
            0
        }
    } else {
        if user_allocation.purchase_count > 0 {
            msg!("Referral bonus skipped - not first purchase");
        }
        0
    };

    // Update user allocation
    user_allocation.user = user.key();
    user_allocation.total_tokens = user_allocation
        .total_tokens
        .checked_add(tokens_to_allocate)
        .ok_or(ErrorCode::Overflow)?;
    user_allocation.total_spent_cents = new_total_spent;
    user_allocation.purchase_count = user_allocation.purchase_count.checked_add(1).ok_or(ErrorCode::Overflow)?;
    
    if user_allocation.purchase_count == 1 {
        user_allocation.first_purchase_at = clock.unix_timestamp;
    }
    user_allocation.last_purchase_at = clock.unix_timestamp;

    // Update stage progress
    config.stage_tokens_sold = config
        .stage_tokens_sold
        .checked_add(tokens_to_allocate)
        .ok_or(ErrorCode::Overflow)?;
    config.tokens_sold = config
        .tokens_sold
        .checked_add(tokens_to_allocate)
        .ok_or(ErrorCode::Overflow)?;
    config.total_usd_raised_cents = config
        .total_usd_raised_cents
        .checked_add(usd_cents)
        .ok_or(ErrorCode::Overflow)?;

    // Advance stage if current stage is full
    while config.stage_tokens_sold >= TOKENS_PER_STAGE * TOKEN_DECIMALS && config.current_stage < 9 {
        config.current_stage += 1;
        config.stage_tokens_sold = config.stage_tokens_sold
            .checked_sub(TOKENS_PER_STAGE * TOKEN_DECIMALS)
            .ok_or(ErrorCode::Overflow)?;
        
        msg!("Advanced to stage {}", config.current_stage + 1);
    }

    // Close presale if hard cap reached
    if config.tokens_sold >= PRESALE_TOTAL_ALLOCATION * TOKEN_DECIMALS {
        config.presale_active = false;
        msg!("Presale hard cap reached - presale closed");
    }

    msg!(
        "{} Allocation recorded: {} tokens to user, {} referral bonus",
        coin_name,
        tokens_to_allocate,
        referral_bonus
    );

    Ok(())
}

/// Internal function for stablecoin (USDT/USDC) purchase with vesting stake
/// Combines stablecoin purchase with automatic vesting stake creation/update
fn presale_purchase_stablecoin_and_vest_stake_internal<'info>(
    config: &mut Account<'info, Config>,
    user_account: &mut Account<'info, UserAccount>,
    user_allocation: &mut Account<'info, PresaleAllocation>,
    stake_account: &mut Account<'info, StakeAccount>,
    stablecoin_ata_for_user: &Account<'info, TokenAccount>,
    stablecoin_ata_for_admin: &Account<'info, TokenAccount>,
    user: &Signer<'info>,
    token_program: &Program<'info, Token>,
    stablecoin_amount: u64,
    tier: StakeTier,
    auto_compound: bool,
    coin_name: &str,
) -> Result<()> {
    let clock = Clock::get()?;

    // Validate presale is active
    require!(config.presale_active, ErrorCode::PresaleNotStarted);
    require!(
        clock.unix_timestamp >= config.presale_start_time,
        ErrorCode::PresaleNotStarted
    );
    require!(
        config.tokens_sold < PRESALE_TOTAL_ALLOCATION * TOKEN_DECIMALS,
        ErrorCode::PresaleHardCapReached
    );

    // Validate user not blocked
    require!(!user_account.is_blocked, ErrorCode::AddressBlocked);
    require!(stablecoin_amount > 0, ErrorCode::InvalidAmount);

    // Convert stablecoin amount to USD cents
    // USDT/USDC have 6 decimals, so 1_000_000 = $1.00 = 100 cents
    let usd_cents = stablecoin_amount
        .checked_mul(100)
        .ok_or(ErrorCode::Overflow)?
        .checked_div(1_000_000)
        .ok_or(ErrorCode::Overflow)?;

    msg!("{} Vesting Stake: {} amount: {}, USD value: {} cents", coin_name, coin_name, stablecoin_amount, usd_cents);

    // Validate purchase limits (use config values if set, otherwise use constants)
    let min_purchase = if config.min_purchase_usd > 0 { config.min_purchase_usd } else { PRESALE_MIN_PURCHASE_USD };
    let max_purchase = if config.max_per_user_usd > 0 { config.max_per_user_usd } else { PRESALE_MAX_PURCHASE_USD };
    
    // Check minimum per transaction
    require!(usd_cents >= min_purchase, ErrorCode::BelowMinimumPurchase);
    
    // Check maximum per transaction ($10,000)
    require!(usd_cents <= max_purchase, ErrorCode::ExceedsMaximumPurchase);
    
    let new_total_spent = user_allocation.total_spent_cents
        .checked_add(usd_cents)
        .ok_or(ErrorCode::Overflow)?;
    
    // Check maximum total per user ($25,600)
    require!(
        new_total_spent <= PRESALE_MAX_TOTAL_PER_USER_USD,
        ErrorCode::ExceedsMaximumPurchase
    );

    // Calculate tokens based on current stage price
    let tokens_to_stake = calculate_tokens_for_usd(usd_cents, config.current_stage)?;

    // Check global staking cap
    require!(
        config.total_staked.checked_add(tokens_to_stake).ok_or(ErrorCode::Overflow)? <= MAX_TOTAL_STAKED,
        ErrorCode::StakingCapReached
    );

    // Validate tier-specific constraints
    let lock_period_days = match tier {
        StakeTier::TierA => {
            require!(
                config.total_staked_tier_a.checked_add(tokens_to_stake).ok_or(ErrorCode::Overflow)? <= MAX_STAKE_TIER_A,
                ErrorCode::TierAFull
            );
            LOCK_PERIOD_TIER_A
        }
        StakeTier::TierB => LOCK_PERIOD_TIER_B,
        StakeTier::TierC => LOCK_PERIOD_TIER_C,
    };

    // Transfer stablecoin from user to admin
    let cpi_ctx = CpiContext::new(
        token_program.to_account_info(),
        Transfer {
            from: stablecoin_ata_for_user.to_account_info(),
            to: stablecoin_ata_for_admin.to_account_info(),
            authority: user.to_account_info(),
        },
    );
    token::transfer(cpi_ctx, stablecoin_amount)?;

    // Update user allocation (record keeping)
    user_allocation.user = user.key();
    user_allocation.total_tokens = user_allocation
        .total_tokens
        .checked_add(tokens_to_stake)
        .ok_or(ErrorCode::Overflow)?;
    user_allocation.total_spent_cents = new_total_spent;
    user_allocation.purchase_count = user_allocation.purchase_count.checked_add(1).ok_or(ErrorCode::Overflow)?;
    
    if user_allocation.purchase_count == 1 {
        user_allocation.first_purchase_at = clock.unix_timestamp;
    }
    user_allocation.last_purchase_at = clock.unix_timestamp;

    // Create or update vesting stake (PDA per user per tier)
    let is_new_stake = stake_account.owner == Pubkey::default();
    
    if is_new_stake {
        // New stake account - initialize
        let stake_id = config.next_stake_id;
        stake_account.stake_id = stake_id;
        stake_account.owner = user.key();
        stake_account.amount = tokens_to_stake;
        stake_account.start_time = clock.unix_timestamp;
        stake_account.lock_period_days = lock_period_days;
        stake_account.last_reward_calculation = clock.unix_timestamp;
        stake_account.pending_rewards = 0;
        stake_account.active = true;
        stake_account.tier = tier;
        stake_account.auto_compound = auto_compound;
        stake_account.cooldown_start = 0;
        stake_account.is_vesting = true;
        stake_account.total_added = tokens_to_stake;
        
        config.next_stake_id = stake_id.checked_add(1).ok_or(ErrorCode::Overflow)?;
        
        msg!("{}: Created new vesting stake: {} tokens in {:?} tier", coin_name, tokens_to_stake, tier);
    } else {
        // Existing stake - calculate pending rewards first, then add new tokens
        let apy = match stake_account.tier {
            StakeTier::TierA => APY_TIER_A,
            StakeTier::TierB => APY_TIER_B,
            StakeTier::TierC => APY_TIER_C,
        };
        
        let time_elapsed = clock.unix_timestamp
            .checked_sub(stake_account.last_reward_calculation)
            .ok_or(ErrorCode::Overflow)?;
        
        if time_elapsed > 0 {
            let rewards = (stake_account.amount as u128)
                .checked_mul(apy as u128)
                .ok_or(ErrorCode::Overflow)?
                .checked_mul(time_elapsed as u128)
                .ok_or(ErrorCode::Overflow)?
                .checked_div(100 * SECONDS_PER_YEAR as u128)
                .ok_or(ErrorCode::Overflow)? as u64;
            
            if stake_account.auto_compound {
                stake_account.amount = stake_account.amount
                    .checked_add(rewards)
                    .ok_or(ErrorCode::Overflow)?;
            } else {
                stake_account.pending_rewards = stake_account.pending_rewards
                    .checked_add(rewards)
                    .ok_or(ErrorCode::Overflow)?;
            }
        }
        
        // Add new tokens to existing stake
        stake_account.amount = stake_account.amount
            .checked_add(tokens_to_stake)
            .ok_or(ErrorCode::Overflow)?;
        stake_account.total_added = stake_account.total_added
            .checked_add(tokens_to_stake)
            .ok_or(ErrorCode::Overflow)?;
        stake_account.last_reward_calculation = clock.unix_timestamp;
        stake_account.auto_compound = auto_compound; // Update auto_compound preference
        
        msg!("{}: Added {} tokens to existing {:?} stake (total: {})", coin_name, tokens_to_stake, tier, stake_account.amount);
    }

    // Update staking config
    config.total_staked = config
        .total_staked
        .checked_add(tokens_to_stake)
        .ok_or(ErrorCode::Overflow)?;
    
    if tier == StakeTier::TierA {
        config.total_staked_tier_a = config
            .total_staked_tier_a
            .checked_add(tokens_to_stake)
            .ok_or(ErrorCode::Overflow)?;
    }

    // Update stage progress
    config.stage_tokens_sold = config
        .stage_tokens_sold
        .checked_add(tokens_to_stake)
        .ok_or(ErrorCode::Overflow)?;
    config.tokens_sold = config
        .tokens_sold
        .checked_add(tokens_to_stake)
        .ok_or(ErrorCode::Overflow)?;
    config.total_usd_raised_cents = config
        .total_usd_raised_cents
        .checked_add(usd_cents)
        .ok_or(ErrorCode::Overflow)?;

    // Advance stage if current stage is full
    while config.stage_tokens_sold >= TOKENS_PER_STAGE * TOKEN_DECIMALS && config.current_stage < 9 {
        config.current_stage += 1;
        config.stage_tokens_sold = config.stage_tokens_sold
            .checked_sub(TOKENS_PER_STAGE * TOKEN_DECIMALS)
            .ok_or(ErrorCode::Overflow)?;
        
        msg!("Advanced to stage {}", config.current_stage + 1);
    }

    // Close presale if hard cap reached
    if config.tokens_sold >= PRESALE_TOTAL_ALLOCATION * TOKEN_DECIMALS {
        config.presale_active = false;
        msg!("Presale hard cap reached - presale closed");
    }

    msg!(
        "{} Vesting Stake complete: {} tokens staked to {:?} tier",
        coin_name,
        tokens_to_stake,
        tier
    );

    Ok(())
}

/// Calculate tokens to allocate based on USD value and current stage
fn calculate_tokens_for_usd(usd_cents: u64, stage: u8) -> Result<u64> {
    if stage >= 10 {
        return Err(ErrorCode::PresaleEnded.into());
    }

    let price_with_4_decimals = STAGE_PRICES[stage as usize]; // e.g., 1501 = $0.1501
    
    // Convert price from 4-decimal format to cents
    // $0.1501 is stored as 1501 (10000x multiplier for 4 decimals)
    // To get price in cents: 1501 / 10000 * 100 = 15.01 cents
    // Simplified: price_cents = price_with_4_decimals / 100
    
    // tokens = (usd_cents * TOKEN_DECIMALS * 100) / price_with_4_decimals
    // Example: $100 at stage 1 ($0.1501)
    // = (10000 cents * 1e9 * 100) / 1501
    // = 666,222,518,320,453 raw units = 666,222.52 tokens
    let tokens = (usd_cents as u128)
        .checked_mul(TOKEN_DECIMALS as u128)
        .ok_or(ErrorCode::Overflow)?
        .checked_mul(100) // Adjust for 4-decimal price format
        .ok_or(ErrorCode::Overflow)?
        .checked_div(price_with_4_decimals as u128)
        .ok_or(ErrorCode::Overflow)?;

    Ok(tokens as u64)
}

fn calculate_rewards_internal(
    _config: &Config,
    stake: &StakeAccount,
    current_time: i64,
) -> Result<u64> {
    require!(stake.active, ErrorCode::StakeNotActive);

    if stake.last_reward_calculation == current_time {
        return Ok(stake.pending_rewards);
    }

    let time_elapsed = current_time
        .checked_sub(stake.last_reward_calculation)
        .ok_or(ErrorCode::Overflow)?;

    // Get APY based on tier
    let apy = match stake.tier {
        StakeTier::TierA => APY_TIER_A,
        StakeTier::TierB => APY_TIER_B,
        StakeTier::TierC => APY_TIER_C,
    };

    // Calculate rewards: (amount * APY * time) / (seconds_per_year * 100)
    let new_rewards = (stake.amount as u128)
        .checked_mul(apy as u128)
        .ok_or(ErrorCode::Overflow)?
        .checked_mul(time_elapsed as u128)
        .ok_or(ErrorCode::Overflow)?
        .checked_div(
            (SECONDS_PER_YEAR as u128)
                .checked_mul(100)
                .ok_or(ErrorCode::Overflow)?,
        )
        .ok_or(ErrorCode::Overflow)? as u64;

    Ok(stake
        .pending_rewards
        .checked_add(new_rewards)
        .ok_or(ErrorCode::Overflow)?)
}

// =====================================================
// ACCOUNT STRUCTURES
// =====================================================

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        seeds = [b"config", admin.key().as_ref()],
        bump,
        space = 8 + Config::SPACE
    )]
    pub config: Account<'info, Config>,

    #[account(
        init,
        payer = admin,
        seeds = [ico_mint.key().as_ref()],
        bump,
        token::mint = ico_mint,
        token::authority = ico_ata_for_ico_program,
    )]
    pub ico_ata_for_ico_program: Account<'info, TokenAccount>,

    pub ico_mint: Account<'info, Mint>,

    #[account(mut)]
    pub ico_ata_for_admin: Account<'info, TokenAccount>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct PresalePurchaseWithSol<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"user", user.key().as_ref()],
        bump,
        space = 8 + UserAccount::SPACE
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"allocation", user.key().as_ref()],
        bump,
        space = 8 + PresaleAllocation::SPACE
    )]
    pub user_allocation: Account<'info, PresaleAllocation>,

    /// CHECK: Optional referrer allocation account - validated manually in function
    #[account(mut)]
    pub referrer_allocation: UncheckedAccount<'info>,

    /// Pyth SOL/USD price account
    pub pyth_sol_usd_price: Account<'info, PriceUpdateV2>,

    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: SOL Treasury receives SOL payments - VALIDATED against config.sol_treasury
    #[account(
        mut,
        constraint = sol_treasury.key() == config.sol_treasury @ ErrorCode::InvalidAdmin
    )]
    pub sol_treasury: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

/// Account struct for presale purchase with automatic vesting stake
#[derive(Accounts)]
#[instruction(sol_amount: u64, tier: StakeTier)]
pub struct PresalePurchaseAndVestStake<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"user", user.key().as_ref()],
        bump,
        space = 8 + UserAccount::SPACE
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"allocation", user.key().as_ref()],
        bump,
        space = 8 + PresaleAllocation::SPACE
    )]
    pub user_allocation: Account<'info, PresaleAllocation>,

    /// Stake account - PDA per user per tier (max 3 accounts per user)
    /// Seeds: ["vesting_stake", user_pubkey, tier_byte]
    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"vesting_stake", user.key().as_ref(), &[tier as u8]],
        bump,
        space = 8 + StakeAccount::SPACE
    )]
    pub stake_account: Account<'info, StakeAccount>,

    /// Pyth SOL/USD price account
    pub pyth_sol_usd_price: Account<'info, PriceUpdateV2>,

    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: SOL Treasury receives SOL payments - VALIDATED against config.sol_treasury
    #[account(
        mut,
        constraint = sol_treasury.key() == config.sol_treasury @ ErrorCode::InvalidAdmin
    )]
    pub sol_treasury: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PresalePurchaseWithStablecoin<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"user", user.key().as_ref()],
        bump,
        space = 8 + UserAccount::SPACE
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"allocation", user.key().as_ref()],
        bump,
        space = 8 + PresaleAllocation::SPACE
    )]
    pub user_allocation: Account<'info, PresaleAllocation>,

    /// CHECK: Optional referrer allocation account - validated manually in function
    #[account(mut)]
    pub referrer_allocation: UncheckedAccount<'info>,

    #[account(mut)]
    pub stablecoin_ata_for_user: Account<'info, TokenAccount>,

    #[account(mut)]
    pub stablecoin_ata_for_admin: Account<'info, TokenAccount>,

    pub stablecoin_mint: Account<'info, Mint>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

/// Account struct for presale purchase with stablecoin (USDT/USDC) and automatic vesting stake
/// Uses PDA per user per tier (max 3 stake accounts per user)
#[derive(Accounts)]
#[instruction(stablecoin_amount: u64, tier: StakeTier)]
pub struct PresalePurchaseStablecoinAndVestStake<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"user", user.key().as_ref()],
        bump,
        space = 8 + UserAccount::SPACE
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"allocation", user.key().as_ref()],
        bump,
        space = 8 + PresaleAllocation::SPACE
    )]
    pub user_allocation: Account<'info, PresaleAllocation>,

    /// Stake account - PDA per user per tier (max 3 accounts per user)
    /// Seeds: ["vesting_stake", user_pubkey, tier_byte]
    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"vesting_stake", user.key().as_ref(), &[tier as u8]],
        bump,
        space = 8 + StakeAccount::SPACE
    )]
    pub stake_account: Account<'info, StakeAccount>,

    #[account(mut)]
    pub stablecoin_ata_for_user: Account<'info, TokenAccount>,

    #[account(mut)]
    pub stablecoin_ata_for_admin: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimPresaleAllocation<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [b"allocation", user.key().as_ref()],
        bump,
    )]
    pub user_allocation: Account<'info, PresaleAllocation>,

    #[account(mut)]
    pub ico_ata_for_ico_program: Account<'info, TokenAccount>,

    #[account(mut)]
    pub ico_ata_for_user: Account<'info, TokenAccount>,

    pub ico_mint: Account<'info, Mint>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

/// Admin claim for user - allows admin to claim tokens on behalf of a user
#[derive(Accounts)]
pub struct AdminClaimForUser<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [b"allocation", user.key().as_ref()],
        bump,
    )]
    pub user_allocation: Account<'info, PresaleAllocation>,

    #[account(mut)]
    pub ico_ata_for_ico_program: Account<'info, TokenAccount>,

    #[account(mut)]
    pub ico_ata_for_user: Account<'info, TokenAccount>,

    pub ico_mint: Account<'info, Mint>,

    /// CHECK: The user account to claim for (not a signer - admin is claiming for them)
    pub user: AccountInfo<'info>,

    /// Admin must be the config admin
    #[account(
        mut,
        constraint = admin.key() == config.admin @ ErrorCode::InvalidAdmin
    )]
    pub admin: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

/// Admin add allocation for giveaways/airdrops
/// Creates or updates allocation without requiring payment
#[derive(Accounts)]
pub struct AdminAddAllocation<'info> {
    /// Config account - contains admin pubkey
    #[account(
        mut,
        seeds = [b"config", admin.key().as_ref()],
        bump,
    )]
    pub config: Account<'info, Config>,

    /// Admin signer - MUST match config.admin
    #[account(mut)]
    pub admin: Signer<'info>,

    /// Recipient wallet (giveaway recipient) - does NOT need to sign
    /// CHECK: This is just the target wallet address for the giveaway
    pub recipient: AccountInfo<'info>,

    /// User allocation PDA - created if doesn't exist
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + PresaleAllocation::SPACE,
        seeds = [b"allocation", recipient.key().as_ref()],
        bump,
    )]
    pub user_allocation: Account<'info, PresaleAllocation>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(tier: StakeTier)]
pub struct ClaimAndStake<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"user", user.key().as_ref()],
        bump,
        space = 8 + UserAccount::SPACE
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        mut,
        seeds = [b"allocation", user.key().as_ref()],
        bump,
    )]
    pub user_allocation: Account<'info, PresaleAllocation>,

    #[account(
        init,
        payer = user,
        seeds = [b"stake", user.key().as_ref(), config.next_stake_id.to_le_bytes().as_ref()],
        bump,
        space = 8 + StakeAccount::SPACE
    )]
    pub stake_account: Account<'info, StakeAccount>,

    #[account(mut)]
    pub ico_ata_for_ico_program: Account<'info, TokenAccount>,

    pub ico_mint: Account<'info, Mint>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RegisterReferrer<'info> {
    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"user", user.key().as_ref()],
        bump,
        space = 8 + UserAccount::SPACE
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"allocation", user.key().as_ref()],
        bump,
        space = 8 + PresaleAllocation::SPACE
    )]
    pub user_allocation: Account<'info, PresaleAllocation>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct StakeTokens<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"user", user.key().as_ref()],
        bump,
        space = 8 + UserAccount::SPACE
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        init,
        payer = user,
        seeds = [b"stake", user.key().as_ref(), config.next_stake_id.to_le_bytes().as_ref()],
        bump,
        space = 8 + StakeAccount::SPACE
    )]
    pub stake_account: Account<'info, StakeAccount>,

    #[account(mut)]
    pub ico_ata_for_user: Account<'info, TokenAccount>,

    #[account(mut)]
    pub ico_ata_for_ico_program: Account<'info, TokenAccount>,

    pub ico_mint: Account<'info, Mint>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ToggleAutoCompound<'info> {
    #[account(mut)]
    pub stake_account: Account<'info, StakeAccount>,

    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct HarvestRewards<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"user", user.key().as_ref()],
        bump,
        space = 8 + UserAccount::SPACE
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(mut)]
    pub stake_account: Account<'info, StakeAccount>,

    #[account(mut)]
    pub ico_ata_for_user: Account<'info, TokenAccount>,

    #[account(mut)]
    pub ico_ata_for_ico_program: Account<'info, TokenAccount>,

    pub ico_mint: Account<'info, Mint>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitiateUnstake<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"user", user.key().as_ref()],
        bump,
        space = 8 + UserAccount::SPACE
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(mut)]
    pub stake_account: Account<'info, StakeAccount>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FinalizeUnstake<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(
        init_if_needed,
        payer = user,
        seeds = [b"user", user.key().as_ref()],
        bump,
        space = 8 + UserAccount::SPACE
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(mut)]
    pub stake_account: Account<'info, StakeAccount>,

    #[account(mut)]
    pub ico_ata_for_user: Account<'info, TokenAccount>,

    #[account(mut)]
    pub ico_ata_for_ico_program: Account<'info, TokenAccount>,

    pub ico_mint: Account<'info, Mint>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        constraint = admin.key() == config.admin @ ErrorCode::InvalidAdmin
    )]
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct ResizeConfig<'info> {
    /// CHECK: We're manually handling the reallocation
    #[account(mut)]
    pub config: AccountInfo<'info>,

    /// Admin must match the config admin - Note: config is AccountInfo so we validate in function
    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetBlockStatus<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(
        init_if_needed,
        payer = admin,
        seeds = [b"user", target_user.key().as_ref()],
        bump,
        space = 8 + UserAccount::SPACE
    )]
    pub user_account: Account<'info, UserAccount>,

    /// CHECK: This is the user we're setting block status for
    pub target_user: AccountInfo<'info>,

    #[account(
        mut,
        constraint = admin.key() == config.admin @ ErrorCode::InvalidAdmin
    )]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawTokens<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(mut)]
    pub ico_ata_for_ico_program: Account<'info, TokenAccount>,

    #[account(mut)]
    pub ico_ata_for_admin: Account<'info, TokenAccount>,

    pub ico_mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = admin.key() == config.admin @ ErrorCode::InvalidAdmin
    )]
    pub admin: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

// =====================================================
// TEAM VESTING ACCOUNT STRUCTURES
// =====================================================

/// Accounts for creating team vesting allocation (admin only)
#[derive(Accounts)]
pub struct CreateTeamVesting<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    /// Team vesting account - PDA per team member
    #[account(
        init,
        payer = admin,
        seeds = [b"team_vesting", team_member.key().as_ref()],
        bump,
        space = 8 + TeamVesting::SPACE
    )]
    pub team_vesting: Account<'info, TeamVesting>,

    /// CHECK: Team member wallet address
    pub team_member: AccountInfo<'info>,

    #[account(
        mut,
        constraint = admin.key() == config.admin @ ErrorCode::InvalidAdmin
    )]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Accounts for claiming team tokens after 18-month lockup
#[derive(Accounts)]
pub struct ClaimTeamTokens<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [b"team_vesting", team_member.key().as_ref()],
        bump,
    )]
    pub team_vesting: Account<'info, TeamVesting>,

    /// Program's token treasury
    #[account(mut)]
    pub ico_ata_for_ico_program: Account<'info, TokenAccount>,

    /// Team member's token account to receive tokens
    #[account(mut)]
    pub team_member_ata: Account<'info, TokenAccount>,

    pub ico_mint: Account<'info, Mint>,

    #[account(mut)]
    pub team_member: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

/// Accounts for viewing team vesting status
#[derive(Accounts)]
pub struct GetTeamVestingStatus<'info> {
    pub config: Account<'info, Config>,

    #[account(
        seeds = [b"team_vesting", team_vesting.member.as_ref()],
        bump,
    )]
    pub team_vesting: Account<'info, TeamVesting>,
}

// =====================================================
// CROSS-CHAIN ACCOUNT STRUCTURES
// =====================================================

/// Accounts for coordinator-initiated transfer and vesting stake for EVM buyers
#[derive(Accounts)]
#[instruction(buyer_eth_address: [u8; 20], chain_id: u8)]
pub struct CoordinatorMintAndVestStake<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    /// The cross-chain allocation to track this purchase
    #[account(
        init_if_needed,
        payer = coordinator,
        seeds = [b"cross_chain", buyer_eth_address.as_ref(), &[chain_id]],
        bump,
        space = 8 + CrossChainAllocation::SPACE
    )]
    pub cross_chain_allocation: Account<'info, CrossChainAllocation>,

    /// Stake account - created for each vesting stake
    #[account(
        init,
        payer = coordinator,
        space = 8 + StakeAccount::SPACE
    )]
    pub stake_account: Account<'info, StakeAccount>,

    /// The Solana wallet that will own the staked tokens
    /// CHECK: This is the beneficiary's Solana address provided by coordinator
    pub beneficiary: AccountInfo<'info>,

    /// Token mint for NOC
    pub ico_mint: Account<'info, Mint>,

    /// Program's token treasury - tokens are transferred FROM here
    /// This is where presale tokens are stored
    #[account(
        mut,
        seeds = [ico_mint.key().as_ref()],
        bump,
    )]
    pub ico_ata_for_ico_program: Account<'info, TokenAccount>,

    /// Stake pool token account (program-owned) - initialized if needed
    #[account(
        init_if_needed,
        payer = coordinator,
        associated_token::mint = ico_mint,
        associated_token::authority = stake_pool_authority,
    )]
    pub stake_pool_ata: Account<'info, TokenAccount>,

    /// Stake pool authority PDA
    /// CHECK: PDA for stake pool
    #[account(
        seeds = [b"stake_pool"],
        bump,
    )]
    pub stake_pool_authority: AccountInfo<'info>,

    /// Coordinator signer
    #[account(mut)]
    pub coordinator: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(buyer_eth_address: [u8; 20], chain_id: u8)]
pub struct RecordCrossChainPurchase<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(
        init_if_needed,
        payer = coordinator,
        seeds = [b"cross_chain", buyer_eth_address.as_ref(), &[chain_id]],
        bump,
        space = 8 + CrossChainAllocation::SPACE
    )]
    pub cross_chain_allocation: Account<'info, CrossChainAllocation>,

    /// CHECK: Optional referrer cross-chain allocation
    #[account(mut)]
    pub referrer_cross_chain_allocation: UncheckedAccount<'info>,

    #[account(mut)]
    pub coordinator: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(eth_address: [u8; 20], chain_id: u8)]
pub struct LinkSolanaWallet<'info> {
    #[account(
        mut,
        seeds = [b"cross_chain", eth_address.as_ref(), &[chain_id]],
        bump,
    )]
    pub cross_chain_allocation: Account<'info, CrossChainAllocation>,

    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClaimCrossChainAllocation<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,

    #[account(mut)]
    pub cross_chain_allocation: Account<'info, CrossChainAllocation>,

    #[account(mut)]
    pub ico_ata_for_ico_program: Account<'info, TokenAccount>,

    #[account(mut)]
    pub ico_ata_for_user: Account<'info, TokenAccount>,

    pub ico_mint: Account<'info, Mint>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

// =====================================================
// DATA STRUCTURES
// =====================================================

#[account]
pub struct Config {
    pub admin: Pubkey,                   // 32
    pub sale_token: Pubkey,              // 32
    pub usdt_address: Pubkey,            // 32
    pub usdc_address: Pubkey,            // 32
    pub sol_price_for_token: u64,        // 8
    pub sol_price_for_stablecoin: u64,   // 8
    pub usdt_ratio: u64,                 // 8
    pub usdc_ratio: u64,                 // 8
    // Presale state
    pub current_stage: u8,               // 1
    pub stage_tokens_sold: u64,          // 8
    pub tokens_sold: u64,                // 8
    pub total_usd_raised_cents: u64,     // 8 - Total USD raised in cents (accurate tracking)
    pub presale_start_time: i64,         // 8
    pub tge_timestamp: i64,              // 8
    pub presale_active: bool,            // 1
    // Staking state
    pub total_penalty_collected: u64,    // 8
    pub min_stake_amount: u64,           // 8
    pub total_staked: u64,               // 8
    pub total_staked_tier_a: u64,        // 8 (new: track Tier A separately)
    pub total_rewards_distributed: u64,  // 8
    pub total_stakers: u64,              // 8
    pub next_stake_id: u64,              // 8
    pub referral_reward_percentage: u64, // 8
    pub total_referral_bonuses: u64,     // 8 - Total referral bonuses issued (from Community Rewards 5%)
    // Cross-chain support
    pub coordinator: Pubkey,             // 32 - Cross-chain coordinator address
    pub cross_chain_tokens_sold: u64,    // 8 - Tokens sold via cross-chain (ETH, BNB)
    // Purchase limits (configurable by admin)
    pub max_per_user_usd: u64,           // 8 - Max purchase per user in cents (0 = use constant)
    pub min_purchase_usd: u64,           // 8 - Min purchase per user in cents (0 = use constant)
    // Treasury for SOL payments (separate from admin for multisig support)
    pub sol_treasury: Pubkey,            // 32 - SOL payments go here (Squads vault)
}

impl Config {
    pub const SPACE: usize = 32 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 1 + 8 + 8 + 8 + 8 + 8 + 1 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 32 + 8 + 8 + 8 + 32; // Added 32 bytes for sol_treasury
}

#[account]
pub struct UserAccount {
    pub is_blocked: bool,      // 1
    pub referrer: Pubkey,      // 32
    pub has_staked: bool,      // 1
    pub total_referrals: u64,  // 8
    pub referral_rewards: u64, // 8
}

impl UserAccount {
    pub const SPACE: usize = 1 + 32 + 1 + 8 + 8;
}

/// Presale allocation record (NOT minted until TGE claim)
#[account]
pub struct PresaleAllocation {
    pub user: Pubkey,                 // 32
    pub total_tokens: u64,            // 8 - tokens they're entitled to
    pub total_spent_cents: u64,       // 8 - USD spent in cents
    pub purchase_count: u32,          // 4
    pub first_purchase_at: i64,       // 8
    pub last_purchase_at: i64,        // 8
    pub referral_bonus_tokens: u64,   // 8 - bonus from referrals
    pub referrer: Pubkey,             // 32 - who referred them
    pub claimed: bool,                // 1 - claimed at TGE?
}

impl PresaleAllocation {
    pub const SPACE: usize = 32 + 8 + 8 + 4 + 8 + 8 + 8 + 32 + 1;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum StakeTier {
    TierA, // 365 days, 128% APY
    TierB, // 182 days, 68% APY
    TierC, // 90 days, 34% APY
}

#[account]
pub struct StakeAccount {
    pub stake_id: u64,                // 8
    pub owner: Pubkey,                // 32
    pub amount: u64,                  // 8
    pub start_time: i64,              // 8
    pub lock_period_days: u64,        // 8
    pub last_reward_calculation: i64, // 8
    pub pending_rewards: u64,         // 8
    pub active: bool,                 // 1
    pub tier: StakeTier,              // 1
    pub auto_compound: bool,          // 1
    pub cooldown_start: i64,          // 8 - timestamp when unstake initiated
    pub is_vesting: bool,             // 1 - true if this is a pre-TGE vesting stake
    pub total_added: u64,             // 8 - total tokens ever added to this stake (for tracking multiple purchases)
}

impl StakeAccount {
    pub const SPACE: usize = 8 + 32 + 8 + 8 + 8 + 8 + 8 + 1 + 1 + 1 + 8 + 1 + 8;
}

// =====================================================
// CROSS-CHAIN DATA STRUCTURES
// =====================================================

/// Cross-chain allocation for ETH/BNB buyers
#[account]
pub struct CrossChainAllocation {
    pub eth_address: [u8; 20],        // 20 - Ethereum/BNB address
    pub chain_id: u8,                 // 1 - 1=Ethereum, 56=BNB, 137=Polygon
    pub total_tokens: u64,            // 8 - tokens allocated
    pub total_usd_cents: u64,         // 8 - USD spent in cents
    pub purchase_count: u32,          // 4
    pub first_purchase_at: i64,       // 8
    pub last_purchase_at: i64,        // 8
    pub referrer_eth: [u8; 20],       // 20 - referrer ETH address
    pub referral_bonus: u64,          // 8 - referral bonus received
    pub linked_solana_wallet: Pubkey, // 32 - linked Solana wallet for claim
    pub claimed: bool,                // 1
}

impl CrossChainAllocation {
    pub const SPACE: usize = 20 + 1 + 8 + 8 + 4 + 8 + 8 + 20 + 8 + 32 + 1;
}

/// Cross-chain referral bonus tracking
#[account]
pub struct CrossChainReferral {
    pub referrer_eth: [u8; 20],       // 20 - referrer ETH address
    pub chain_id: u8,                 // 1
    pub total_bonus: u64,             // 8 - total bonus earned
    pub referral_count: u32,          // 4
    pub linked_solana_wallet: Pubkey, // 32 - linked Solana wallet
    pub claimed: bool,                // 1
}

impl CrossChainReferral {
    pub const SPACE: usize = 20 + 1 + 8 + 4 + 32 + 1;
}

// =====================================================
// TEAM VESTING DATA STRUCTURES
// =====================================================

/// Team member vesting allocation - locked for 18 months after TGE
#[account]
pub struct TeamVesting {
    pub member: Pubkey,           // 32 - team member wallet
    pub total_allocation: u64,    // 8 - total tokens allocated
    pub claimed_amount: u64,      // 8 - tokens already claimed
    pub created_at: i64,          // 8 - when allocation was created
    pub cliff_end: i64,           // 8 - when 18-month lockup ends (TGE + 18 months)
    pub is_active: bool,          // 1 - is allocation active
}

impl TeamVesting {
    pub const SPACE: usize = 32 + 8 + 8 + 8 + 8 + 1;
}
