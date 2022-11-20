import { expect } from "chai";
import { before } from "mocha";
import { QueryMsg, ExecuteMsg } from "client/Identityservice.types.js";

import { createClient } from "../lib/createClient.js";
import { createUser } from "../lib/createUser.js";

import { readContractAddrs } from "../lib/readContractAddrs.js";
import { IdentityserviceClient, IdentityserviceQueryClient } from "../client/Identityservice.client.js";

const client = (await createClient()) as any;

const user1 = createUser(process.env.USER1_MNEMONIC);
const user1_name = process.env.USER1_NAME;

const user2 = createUser(process.env.USER2_MNEMONIC);
const user2_name = process.env.USER2_NAME;

const user3 = createUser(process.env.USER3_MNEMONIC);
const user3_name = process.env.USER3_NAME;

global.liveAddrs = {};

let queryClient: IdentityserviceQueryClient;

describe("User Identity", function () {
  before(async function () {
    global.addrs = await readContractAddrs();
    queryClient = new IdentityserviceQueryClient(client, global.addrs.identityservice);
  });
  it("should register a user identity with valid name", async function () {
    const contract_addr = global.addrs.identityservice;
    const msg: ExecuteMsg = { register_user: { name: user1_name } };

    const executeClient: IdentityserviceClient = new IdentityserviceClient(client, user1, contract_addr);
    const result = await executeClient.registerUser(msg.register_user);

    expect(result['code']).to.equal(0);

    return result;
  });
  it("should return error when registering a user identity with name already taken", async function () {
    const contract_addr = global.addrs.identityservice;
    const msg: ExecuteMsg = { register_user: { name: user1_name } };

    let error;

    try {
      const executeClient: IdentityserviceClient = new IdentityserviceClient(client, user2, contract_addr);
      const result = await executeClient.registerUser(msg.register_user);
    } catch (e) {
      error = e;
    }

    expect(error.response.data.code).to.equal(3);
    expect(error.response.data.message).to.include("Name has been taken");
  });
  it("should resolve a user identity by owner address", async function () {
    const query: QueryMsg = {
      get_identity_by_owner: {
        owner: user1.address,
      },
    };

    const result = await queryClient.getIdentityByOwner(query.get_identity_by_owner);

    expect(result.identity.name).to.equal(user1_name);
    expect(result.identity.owner).to.equal(user1.address);

    return result;
  });
  it("should resolve a user identity by name", async function () {
    const contract_addr = global.addrs.identityservice;
    const query: QueryMsg = {
      get_identity_by_name: {
        name: user1_name,
      },
    };

    const result = await queryClient.getIdentityByName(query.get_identity_by_name);

    expect(result.identity.name).to.equal(user1_name);
    expect(result.identity.id_type).to.equal("user");
    expect(result.identity.owner).to.equal(user1.address);

    return result;
  });
  it("should return null when resolving a non-existing user identity by owner address", async function () {
    const contract_addr = global.addrs.identityservice;
    const query: QueryMsg = {
      get_identity_by_owner: {
        owner: "impossible",
      },
    };

    const result = await queryClient.getIdentityByOwner(query.get_identity_by_owner);

    expect(result.identity).to.equal(null);

    return result;
  });
  it("should return null when resolving a non-existing user identity by name", async function () {
    const contract_addr = global.addrs.identityservice;
    const query: QueryMsg = {
      get_identity_by_name: {
        name: "impossible",
      },
    };

    const result = await queryClient.getIdentityByName(query.get_identity_by_name);

    expect(result.identity).to.equal(null);

    return result;
  });
  it("should register a dao identity with valid name", async function () {
    const contract_addr = global.addrs.identityservice;
    const identityClient = new IdentityserviceClient(client, user2, contract_addr);

    const result = await identityClient.registerDao({
      daoName: user2_name,
      members: [
        {
          addr: user1.address,
          weight: 1,
        },
        {
          addr: user2.address,
          weight: 1,
        },
      ],
      thresholdPercentage: "0.51"
      ,
      maxVotingPeriod: {
        height: 1180000,
      },
    });

    expect(result['code']).to.equal(0);

    return result;

  });
  it("should register a user identity with valid name", async function () {
    const contract_addr = global.addrs.identityservice;

    const identityClient = new IdentityserviceClient(client, user2, contract_addr);
    const result = await identityClient.registerUser({ name: user2_name + "user" });

    expect(result['code']).to.equal(0);

    return result;
  });
  it("should register a dao identity with valid name", async function () {
    const contract_addr = global.addrs.identityservice;
    const identityClient = new IdentityserviceClient(client, user2, contract_addr);

    const result = await identityClient.registerDao({
      daoName: user2_name + "another_dao",
      members: [
        {
          addr: user1.address,
          weight: 1,
        },
        {
          addr: user2.address,
          weight: 1,
        },
      ],
      thresholdPercentage: "0.51",
      maxVotingPeriod: {
        height: 1180000,
      },
    });

    expect(result['code']).to.equal(0);

    return result;

  });
  it("should list the daos descending", async function () {
    const result = await queryClient.daos({ order: 'descending' });
    expect(result['daos'][0][0]).to.equal(2);
  })
  it("should list the daos with pagination", async function () {
    const result = await queryClient.daos({ startAfter: 1 });
    expect(result['daos'][0][0]).to.equal(2);
  })
});
