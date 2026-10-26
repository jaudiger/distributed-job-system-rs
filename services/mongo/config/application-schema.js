const application = db.getSiblingDB("application");

const validators = {
  job: {
    $jsonSchema: {
      bsonType: "object",
      required: ["_id", "operations"],
      properties: {
        _id: { bsonType: "objectId" },
        operations: { bsonType: ["int", "long"], minimum: 1 },
      },
    },
  },
  operation: {
    $jsonSchema: {
      bsonType: "object",
      required: ["_id", "job_id", "request"],
      properties: {
        _id: { bsonType: "objectId" },
        job_id: { bsonType: "string" },
        request: { bsonType: "string" },
        result: { bsonType: "string" },
      },
    },
  },
};

for (const [collectionName, validator] of Object.entries(validators)) {
  const collectionInfo = application.getCollectionInfos({ name: collectionName })[0];

  if (!collectionInfo) {
    assert.commandWorked(
      application.runCommand({
        create: collectionName,
        validator,
        validationLevel: "constraint",
        validationAction: "error",
      }),
    );
    continue;
  }

  const options = collectionInfo.options ?? {};
  const currentValidator = options.validator ?? {};
  const currentAction = options.validationAction ?? "error";
  if (
    options.validationLevel === "constraint" &&
    EJSON.stringify(currentValidator) === EJSON.stringify(validator) &&
    currentAction === "error"
  ) {
    continue;
  }

  if (options.validationLevel === "constraint") {
    assert.commandWorked(
      application.runCommand({ collMod: collectionName, validationLevel: "strict" }),
    );
  }

  assert.commandWorked(
    application.runCommand({
      collMod: collectionName,
      validator,
      validationLevel: "strict",
      validationAction: "error",
    }),
  );
  assert.commandWorked(
    application.runCommand({ collMod: collectionName, prepareConstraintValidationLevel: true }),
  );
  assert.commandWorked(
    application.runCommand({
      collMod: collectionName,
      validationLevel: "constraint",
      validationAction: "error",
    }),
  );
}
