/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.30.1.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

export type Addr = string;
export interface ConfigResponse {
  art_nft_address?: Addr | null;
  art_nft_name: string;
  art_nft_symbol: string;
  identityservice_contract: Addr;
  owner: Addr;
  [k: string]: unknown;
}
export type ExecuteMsg = {
  mint_art: {
    metadata?: Metadata | null;
    owner: string;
    token_id: string;
    token_uri?: string | null;
    [k: string]: unknown;
  };
} | {
  approve_dealer: {
    approved: number;
    dao: Addr;
    duration: number;
    [k: string]: unknown;
  };
} | {
  revoke_dealer: {
    dao: Addr;
    [k: string]: unknown;
  };
};
export interface Metadata {
  animation_url?: string | null;
  attributes?: Trait[] | null;
  background_color?: string | null;
  description?: string | null;
  external_url?: string | null;
  image?: string | null;
  image_data?: string | null;
  name?: string | null;
  youtube_url?: string | null;
}
export interface Trait {
  display_type?: string | null;
  trait_type: string;
  value: string;
}
export interface InstantiateMsg {
  art_nft_code_id: number;
  art_nft_name: string;
  art_nft_symbol: string;
  identityservice_contract: Addr;
  owner: Addr;
  [k: string]: unknown;
}
export type QueryMsg = {
  get_config: {
    [k: string]: unknown;
  };
};