import * as anchor from "@coral-xyz/anchor";
import { Connection, Keypair, clusterApiUrl } from "@solana/web3.js";
import { readFileSync } from "fs";
import * as dotenv from "dotenv";

// Load environment variables from .env file
dotenv.config();

export function loadWallet(): anchor.Wallet {
    // Check if WALLET_PATH environment variable is set in .env file
    if (!process.env.WALLET_PATH) {
        throw new Error(
            "WALLET_PATH environment variable is not set in .env file. " +
            "Please add WALLET_PATH=/path/to/your/wallet-keypair.json to your .env file."
        );
    }
    
    try {
        const secretKey = Uint8Array.from(
            JSON.parse(readFileSync(process.env.WALLET_PATH, "utf8"))
        );
        return new anchor.Wallet(Keypair.fromSecretKey(secretKey));
    } catch (error) {
        throw new Error(
            `Failed to load wallet from path specified in .env file (${process.env.WALLET_PATH}): ${error.message}`
        );
    }
}

export async function loadProgram() {
    const connection = new Connection(clusterApiUrl("devnet"), "confirmed");
    const wallet = loadWallet();
    
    // Initialize the provider
    const provider = new anchor.AnchorProvider(connection, wallet, {
        preflightCommitment: "processed",
    });

    // Set the provider globally (optional, but recommended)
    anchor.setProvider(provider);

    // Load the IDL from the JSON file
    const idl = JSON.parse(readFileSync("./target/idl/trade_fun.json", "utf8"));

    // Corrected constructor call: Passing only provider since IDL already contains program ID
    return new anchor.Program(idl, provider);
}
