import { readNewContractChecksums } from "./readNewContractChecksums.js";
import { readCachedContractChecksums } from "./readCachedContractChecksums.js";

function findNewContractFiles(path) {
  const cachedContractChecksums = readCachedContractChecksums();

  const newContractChecksums = readNewContractChecksums(path);

  const newContractFiles = [];

  for (const contract in newContractChecksums) {
    if (Object.hasOwnProperty.call(newContractChecksums, contract)) {
      const cachedChecksum = cachedContractChecksums[contract];
      const newChecksum = newContractChecksums[contract];

      if (cachedChecksum !== newChecksum)
        newContractFiles.push([contract, newChecksum]);
    }
  }

  return newContractFiles;
}

export { findNewContractFiles };
