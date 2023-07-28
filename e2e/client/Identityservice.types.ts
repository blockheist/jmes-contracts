/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.33.0.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

export type Addr = string;
export interface DaosResponse {
  daos: [number, Addr][];
  [k: string]: unknown;
}
export type ExecuteMsg = {
  register_user: {
    name: string;
    [k: string]: unknown;
  };
} | {
  register_dao: RegisterDaoMsg;
};
export type Duration = {
  height: number;
} | {
  time: number;
};
export interface RegisterDaoMsg {
  dao_name: string;
  max_voting_period: Duration;
  members: Member[];
  threshold_percentage: number;
  [k: string]: unknown;
}
export interface Member {
  addr: string;
  weight: number;
}
export type IdType = "user" | "dao";
export interface GetIdentityByNameResponse {
  identity?: Identity | null;
  [k: string]: unknown;
}
export interface Identity {
  id_type: IdType;
  name: string;
  owner: Addr;
  [k: string]: unknown;
}
export interface GetIdentityByOwnerResponse {
  identity?: Identity | null;
  [k: string]: unknown;
}
export interface InstantiateMsg {
  dao_members_code_id: number;
  dao_multisig_code_id: number;
  governance_addr: Addr;
  owner: Addr;
  [k: string]: unknown;
}
export type QueryMsg = {
  get_identity_by_owner: {
    owner: string;
    [k: string]: unknown;
  };
} | {
  get_identity_by_name: {
    name: string;
    [k: string]: unknown;
  };
} | {
  daos: {
    limit?: number | null;
    order?: Ordering | null;
    start_after?: number | null;
    [k: string]: unknown;
  };
};
export type Ordering = "ascending" | "descending";