import { MsgStoreCode } from "@terra-money/terra.js";
import * as fs from "fs";
import { sleep } from "./sleep.js";
import { executeMsg } from "./executeMsg.js";
import { findNewContractFiles } from "./findNewContractFiles.js";
import { readCachedContractChecksums } from "./readCachedContractChecksums.js";
import { writeCachedContractChecksums } from "./writeCachedContractChecksums.js";
import { getAttribute } from "../../e2e/lib/getAttribute.js";
import { readCodeIds } from "../../e2e/lib/readCodeIds.js";

function createStoreMsg(contract, user) {
  const wasm = fs.readFileSync(contract, {
    highWaterMark: 16,
    encoding: "base64",
  });
  return new MsgStoreCode(user.address, wasm);
}

function getContractNameFromPath(path) {
  let regex = RegExp(/artifacts\/(.*?)\.wasm/, "i");
  return path.match(regex)[1];
}

function getCodeIdFromResult(result) {
  return parseInt(getAttribute(result, "store_code", "code_id"));
}

async function uploadContracts(client, user) {
  const contractPath = "../artifacts/";

  const codeIds = readCodeIds();

  const cachedContractChecksums = readCachedContractChecksums();

  const newContractList = findNewContractFiles(contractPath);

  if (newContractList.length > 0)
    console.log("-> Found new contract checksums:", newContractList);

  for (const idx in newContractList) {
    const [contract, checksum] = newContractList[idx];

    const path = contractPath + contract;

    const storeMsg = createStoreMsg(path, user);

    console.log(`-> Storing ${path}*`);
    try {
      const result = await executeMsg(client, storeMsg, user.wallet);

      // update contract checksum cache so we don't need to upload it again if another contract upload times out
      cachedContractChecksums[contract] = checksum;
      writeCachedContractChecksums(cachedContractChecksums);

      // update codeIds
      codeIds[getContractNameFromPath(path)] = getCodeIdFromResult(result);
      fs.writeFileSync(
        `configs/codeIds_${process.env.NETWORK_ENV}.json`,
        JSON.stringify(codeIds),
        "utf8"
      );

      await sleep(5000); // Wait for blockchain propagation to avoid exiting with error
    } catch (err) {
      console.error(err.message);
      throw err;
    }
  }

  console.log(
    "-> Storing contract wasm files finished!",
    JSON.stringify(codeIds)
  );
}

export { uploadContracts };
