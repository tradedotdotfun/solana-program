import { TradeFun } from "../target/types/trade_fun"; // Ensure this path is correct
import { loadProgram } from "./utils";
import * as anchor from "@coral-xyz/anchor";

async function main() {
    const program = (await loadProgram()) as anchor.Program<TradeFun>;
    const provider = program.provider as anchor.AnchorProvider;

    // Find the PDA for Vault and VaultData
    const [vaultPDA] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("vault")],
        program.programId
    );
    const [vaultDataPDA] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("vault_data")],
        program.programId
    );


    try {
        // Send transaction to deposit SOL
        const tx: string = await program.methods
            .depositSol()
            .accounts({
                user: provider.wallet.publicKey,
                vault: vaultPDA,
                vaultData: vaultDataPDA,
                systemProgram: anchor.web3.SystemProgram.programId,
            } as any)
            .rpc();

        console.log("✅ Successfully deposited SOL!");
        console.log("📜 Transaction Signature:", tx);
        console.log("🏦 Vault PDA:", vaultPDA.toBase58());
    } catch (error) {
        console.error("❌ Error depositing SOL:", error);
    }
}

main().catch(console.error);
