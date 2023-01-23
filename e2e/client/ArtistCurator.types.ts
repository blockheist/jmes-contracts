/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.24.0.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

export type Addr = string;
export type Uint128 = string;
export interface ConfigResponse {
  cw20_address: Addr;
  cw721_address?: Addr | null;
  extension?: Empty | null;
  max_tokens: number;
  name: string;
  owner: Addr;
  symbol: string;
  token_uri: string;
  unit_price: Uint128;
  total_tokens_minted: number;
  [k: string]: unknown;
}
export interface Empty {
  [k: string]: unknown;
}
export type ExecuteMsg = {
  receive: Cw20ReceiveMsg;
};
export type Binary = string;
export interface Cw20ReceiveMsg {
  amount: Uint128;
  msg: Binary;
  sender: string;
  [k: string]: unknown;
}
export interface InstantiateMsg {
  cw20_address: Addr;
  extension?: Empty | null;
  max_tokens: number;
  name: string;
  owner: Addr;
  symbol: string;
  token_code_id: number;
  token_uri: string;
  unit_price: Uint128;
  [k: string]: unknown;
}
export type QueryMsg = {
  get_config: {
    [k: string]: unknown;
  };
};