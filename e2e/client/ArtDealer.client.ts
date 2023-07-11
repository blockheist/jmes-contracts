/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.30.1.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

import { CosmWasmClient, SigningCosmWasmClient, ExecuteResult } from "@cosmjs/cosmwasm-stargate";
import { Coin, StdFee } from "@cosmjs/amino";
import { Addr, ConfigResponse, ExecuteMsg, Metadata, Trait, InstantiateMsg, QueryMsg } from "./ArtDealer.types";
export interface ArtDealerReadOnlyInterface {
  contractAddress: string;
  getConfig: () => Promise<GetConfigResponse>;
}
export class ArtDealerQueryClient implements ArtDealerReadOnlyInterface {
  client: CosmWasmClient;
  contractAddress: string;

  constructor(client: CosmWasmClient, contractAddress: string) {
    this.client = client;
    this.contractAddress = contractAddress;
    this.getConfig = this.getConfig.bind(this);
  }

  getConfig = async (): Promise<GetConfigResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      get_config: {}
    });
  };
}
export interface ArtDealerInterface extends ArtDealerReadOnlyInterface {
  contractAddress: string;
  sender: string;
  mintArt: ({
    metadata,
    owner,
    tokenId,
    tokenUri
  }: {
    metadata?: Metadata;
    owner: string;
    tokenId: string;
    tokenUri?: string;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  approveDealer: ({
    approved,
    dao,
    duration
  }: {
    approved: number;
    dao: Addr;
    duration: number;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  revokeDealer: ({
    dao
  }: {
    dao: Addr;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
}
export class ArtDealerClient extends ArtDealerQueryClient implements ArtDealerInterface {
  client: SigningCosmWasmClient;
  sender: string;
  contractAddress: string;

  constructor(client: SigningCosmWasmClient, sender: string, contractAddress: string) {
    super(client, contractAddress);
    this.client = client;
    this.sender = sender;
    this.contractAddress = contractAddress;
    this.mintArt = this.mintArt.bind(this);
    this.approveDealer = this.approveDealer.bind(this);
    this.revokeDealer = this.revokeDealer.bind(this);
  }

  mintArt = async ({
    metadata,
    owner,
    tokenId,
    tokenUri
  }: {
    metadata?: Metadata;
    owner: string;
    tokenId: string;
    tokenUri?: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      mint_art: {
        metadata,
        owner,
        token_id: tokenId,
        token_uri: tokenUri
      }
    }, fee, memo, _funds);
  };
  approveDealer = async ({
    approved,
    dao,
    duration
  }: {
    approved: number;
    dao: Addr;
    duration: number;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      approve_dealer: {
        approved,
        dao,
        duration
      }
    }, fee, memo, _funds);
  };
  revokeDealer = async ({
    dao
  }: {
    dao: Addr;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      revoke_dealer: {
        dao
      }
    }, fee, memo, _funds);
  };
}