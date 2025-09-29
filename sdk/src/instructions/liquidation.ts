import { PublicKey, Transaction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { AuraLendClient } from '../client';
import BN from 'bn.js';

export interface LiquidateObligationParams {
  obligation: PublicKey;
  repayReserve: PublicKey;
  withdrawReserve: PublicKey;
  amount: BN;
}

export class LiquidationInstructions {
  constructor(private client: AuraLendClient) {}

  async liquidateObligation(params: LiquidateObligationParams): Promise<Transaction> {
    const marketPda = this.client.getMarketAddress();

    return this.client.program.methods
      .liquidateObligation(params.amount)
      .accounts({
        market: marketPda,
        obligation: params.obligation,
        repayReserve: params.repayReserve,
        withdrawReserve: params.withdrawReserve,
        liquidator: this.client.wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();
  }
}