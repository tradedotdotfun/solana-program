use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

declare_id!("CoFf4ZpbTJRoPxdJ64JvMn4pVR1wjhvARc8ed91i9i37");

#[program]
pub mod trade_fun {
    use super::*;

    /// Initialize the admin account (Only called once)
    pub fn initialize_admin(ctx: Context<InitializeAdmin>) -> Result<()> {
        let admin_config = &mut ctx.accounts.admin_config;
        admin_config.admin = *ctx.accounts.admin.key;
        msg!("Admin initialized: {}", admin_config.admin);
        Ok(())
    }

    /// Update the admin (Only current admin can update)
    pub fn update_admin(ctx: Context<UpdateAdmin>) -> Result<()> {
        let admin_config = &mut ctx.accounts.admin_config;
        require!(
            admin_config.admin == *ctx.accounts.current_admin.key,
            VaultError::Unauthorized
        );

        admin_config.admin = *ctx.accounts.new_admin.key;
        msg!("Admin updated to: {}", admin_config.admin);
        Ok(())
    }

    /// Initialize the vault (Only admin can call)
    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        reward_ratios: Vec<u64>,
        platform_fee: u64,
    ) -> Result<()> {
        let admin_config = &ctx.accounts.admin_config;
        require!(
            admin_config.admin == *ctx.accounts.admin.key,
            VaultError::Unauthorized
        );

        require!(
            reward_ratios.iter().sum::<u64>() + platform_fee == 100,
            VaultError::InvalidRatioSum
        );

        let vault_data = &mut ctx.accounts.vault_data;
        vault_data.owner = *ctx.accounts.admin.key;
        vault_data.reward_ratios = reward_ratios;
        vault_data.platform_fee = platform_fee;
        vault_data.is_running = false;

        Ok(())
    }

    pub fn update_vault_settings(
        ctx: Context<UpdateVaultSettings>,
        new_reward_ratios: Vec<u64>,
        new_platform_fee: u64,
    ) -> Result<()> {
        let admin_config = &ctx.accounts.admin_config;
        require!(
            admin_config.admin == *ctx.accounts.admin.key,
            VaultError::Unauthorized
        );
    
        require!(
            new_reward_ratios.iter().sum::<u64>() + new_platform_fee == 100,
            VaultError::InvalidRatioSum
        );
    
        let vault_data = &mut ctx.accounts.vault_data;
        vault_data.reward_ratios = new_reward_ratios;
        vault_data.platform_fee = new_platform_fee;
    
        msg!(
            "Updated vault settings: reward ratios = {:?}, platform fee = {}%",
            vault_data.reward_ratios,
            vault_data.platform_fee
        );
    
        Ok(())
    }

    /// Start a new league round (Only admin)
    pub fn start_round(ctx: Context<ManageRound>) -> Result<()> {
        let vault_data = &mut ctx.accounts.vault_data;
        require!(vault_data.owner == *ctx.accounts.admin.key, VaultError::Unauthorized);

        vault_data.is_running = true;
        msg!("League round started!");
        Ok(())
    }

    /// End the current league round (Only admin)
    pub fn end_round(ctx: Context<EndRound>) -> Result<()> {
        let vault_data = &mut ctx.accounts.vault_data;
        require!(vault_data.owner == *ctx.accounts.admin.key, VaultError::Unauthorized);

        vault_data.is_running = false;

        let vault_balance = ctx.accounts.vault.to_account_info().lamports();
        require!(vault_balance > 0, VaultError::InsufficientFunds);

        let platform_fee_amount = vault_balance * vault_data.platform_fee / 100;

        let transfer_instruction = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.admin.to_account_info(),
        };

        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                transfer_instruction,
                &[&[b"vault", &[ctx.bumps.vault]]],
            ),
            platform_fee_amount,
        )?;

        msg!("Platform fee of {} lamports sent to admin.", platform_fee_amount);
        Ok(())
    }

    /// Deposit SOL into the vault (Only allowed if the league is running)
    pub fn deposit_sol(ctx: Context<DepositSol>) -> Result<()> {
        let vault_data = &ctx.accounts.vault_data;
        require!(vault_data.is_running, VaultError::LeagueNotRunning);
    
        
    
        let transfer_instruction = Transfer {
            from: ctx.accounts.user.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
        };
    
        transfer(
            CpiContext::new(ctx.accounts.system_program.to_account_info(), transfer_instruction),
            100_000_000
        )?;

        msg!(
            "Emitting DepositEvent: user={}, timestamp={}",
            ctx.accounts.user.key(),
            Clock::get()?.unix_timestamp
        );
    
        emit!(DepositEvent {
            user: ctx.accounts.user.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });
    
    
        Ok(())
    }

    /// Distribute SOL dynamically based on preset ratios (Only admin)
    pub fn distribute_sol<'info>(ctx: Context<'_, '_, '_, 'info, DistributeSol<'info>>) -> Result<()> {
        let vault_data = &ctx.accounts.vault_data;
        require!(vault_data.owner == *ctx.accounts.admin.key, VaultError::Unauthorized);

        let vault_balance = ctx.accounts.vault.to_account_info().lamports();
        require!(vault_balance > 0, VaultError::InsufficientFunds);

        let recipient_count = ctx.remaining_accounts.len();
        require!(
            recipient_count == vault_data.reward_ratios.len(),
            VaultError::MismatchedRecipients
        );

        for (i, recipient_account) in ctx.remaining_accounts.iter().enumerate() {
            let reward_amount = vault_balance * vault_data.reward_ratios[i] / 100;

            let transfer_instruction = Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: recipient_account.to_account_info(),
            };

            transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.system_program.to_account_info(),
                    transfer_instruction,
                    &[&[b"vault", &[ctx.bumps.vault]]],
                ),
                reward_amount,
            )?;
        }

        Ok(())
    }
}

/// Admin account (Stores the current admin)
#[account]
pub struct AdminConfig {
    pub admin: Pubkey,
}

/// Vault metadata account
#[account]
pub struct VaultData {
    pub owner: Pubkey,
    pub reward_ratios: Vec<u64>,
    pub platform_fee: u64,
    pub is_running: bool,
}

/// Error definitions
#[error_code]
pub enum VaultError {
    #[msg("Unauthorized access! Only the admin can perform this action.")]
    Unauthorized,
    #[msg("The sum of reward ratios and platform fee must be 100.")]
    InvalidRatioSum,
    #[msg("Vault does not have enough SOL to distribute.")]
    InsufficientFunds,
    #[msg("Number of recipients must match number of reward ratios.")]
    MismatchedRecipients,
    #[msg("Deposits are only allowed when the league is running.")]
    LeagueNotRunning,
}

#[derive(Accounts)]
pub struct UpdateVaultSettings<'info> {
    #[account(mut, seeds = [b"vault_data"], bump)]
    pub vault_data: Account<'info, VaultData>,
    #[account(seeds = [b"admin_config"], bump)]
    pub admin_config: Account<'info, AdminConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
}

/// Account Structures
#[derive(Accounts)]
pub struct InitializeAdmin<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + 32, 
        seeds = [b"admin_config"],
        bump
    )]
    pub admin_config: Account<'info, AdminConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateAdmin<'info> {
    #[account(mut, seeds = [b"admin_config"], bump)]
    pub admin_config: Account<'info, AdminConfig>,
    #[account(mut)]
    pub current_admin: Signer<'info>,
    #[account(mut)]
    pub new_admin: Signer<'info>,
}
#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(mut, seeds = [b"admin_config"], bump)]
    pub admin_config: Account<'info, AdminConfig>,
    #[account(
        init,
        payer = admin,
        space = 8 + 32 + (8 * 10) + 8 + 1,
        seeds = [b"vault_data"],
        bump
    )]
    pub vault_data: Account<'info, VaultData>,
    /// CHECK: This is safe because we're using it as a PDA for vault
    #[account(
        seeds = [b"vault"],
        bump
    )]
    pub vault: AccountInfo<'info>,
    #[account(mut)]
    pub admin: Signer<'info>, 
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ManageRound<'info> {
    #[account(mut, seeds = [b"vault_data"], bump)]
    pub vault_data: Account<'info, VaultData>,
    #[account(mut)]
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct EndRound<'info> {
    #[account(mut, seeds = [b"vault_data"], bump)]
    pub vault_data: Account<'info, VaultData>,
    /// CHECK: This is safe because we're using it as a PDA for vault
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: AccountInfo<'info>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositSol<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: This is safe because we're using it as a PDA for vault
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: AccountInfo<'info>,
    #[account(seeds = [b"vault_data"], bump)]
    pub vault_data: Account<'info, VaultData>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DistributeSol<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: This is safe because we're using it as a PDA for vault
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: AccountInfo<'info>,
    #[account(mut, seeds = [b"vault_data"], bump)]
    pub vault_data: Account<'info, VaultData>,
    pub system_program: Program<'info, System>,
}


#[event]
pub struct DepositEvent {
    pub user: Pubkey,
    pub timestamp: i64,
}