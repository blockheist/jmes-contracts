
import { expect } from "chai";
import { before } from "mocha";

import { createClient } from "../lib/createClient.js";
import { createUser } from "../lib/createUser.js";
import { readContractAddrs } from "../lib/readContractAddrs.js";
import { sleep } from "../lib/sleep.js";

import { DistributionClient, DistributionQueryClient } from "../client/Distribution.client.js";

const client = (await createClient()) as any;

const user1 = createUser(process.env.USER1_MNEMONIC);
const user1_name = process.env.USER1_NAME;

const user2 = createUser(process.env.USER2_MNEMONIC);
const user2_name = process.env.USER2_NAME;

const user3 = createUser(process.env.USER3_MNEMONIC);
const user3_name = process.env.USER3_NAME;


describe("Distribution", function () {
  before(async function () {
    global.addrs = await readContractAddrs();
  });
  it("should add funds to the distribution contract", async function () {
    const result = await client.send(user1, global.addrs.distribution, "1000000uluna");
    return result;
  });

  it("should add a grant", async function () {
    const executeClient = new DistributionClient(client, user1, global.addrs.distribution);

    try {
      const result = await executeClient.addGrant({
        dao: "terra1dgnuthnv365t4ltcxpwn734yxmvcw4uy3dk0uq",
        amount: "1000000",
        duration: 300,
      })

      console.log("result :>> ", result);

      return result;
    } catch (e) {
      console.error(e)
      throw e
    }
  });

  it("should query the contract Config", async function () {
    const queryClient = new DistributionQueryClient(client, global.addrs.distribution);

    const result = await queryClient.config();

    console.log("result :>> ", result);

    return result;
  });
  it("should query a grant by grant_id", async function () {
    const queryClient = new DistributionQueryClient(client, global.addrs.distribution);

    const result = await queryClient.grant({ grantId: 1 });

    console.log("result :>> ", result);

    return result;
  });
  it("should query all grants paginated, optionally filtered by dao address", async function () {
    const queryClient = new DistributionQueryClient(client, global.addrs.distribution);

    const result = await queryClient.grants({
      // dao: user1.address,
      startAfter: "0",
      limit: 2,
    });

    console.log("result :>> ", result);

    return result;
  });
  it("should claim the matured portion of the grant", async function () {
    const executeClient = new DistributionClient(client, user1, global.addrs.distribution);

    await sleep(10000);

    const result = await executeClient.claim({ grantId: 1 });

    console.log("result :>> ", result);

    return result;
  });
});
