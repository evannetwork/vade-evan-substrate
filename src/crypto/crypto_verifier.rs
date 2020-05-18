pub mod verifier {
  extern crate ursa;

  use ursa::cl::issuer::Issuer as CryptoIssuer;
  use ursa::cl::verifier::Verifier as CryptoVerifier;
  use ursa::bn::BigNumber;
  use ursa::cl::CredentialPublicKey;
  use ursa::cl::SubProofRequest;
  use ursa::cl::Proof as CryptoProof;
  use ursa::cl::SubProof;

  use std::error::Error;

  use crate::crypto::crypto_datatypes::CryptoProofRequest;

  use crate::application::datatypes::{
    ProofPresentation,
    ProofRequest,
    CredentialDefinition,
    CredentialSchema
  };
  use std::collections::HashMap;


  pub struct CredVerifier {

  }

  impl CredVerifier {

    pub fn new() -> CredVerifier {
      CredVerifier {
      }
    }

    pub fn request_proof(attributes: Vec<&str>) -> SubProofRequest {
      let mut sub_proof_request_builder = CryptoVerifier::new_sub_proof_request_builder().unwrap();
      for i in 0 .. attributes.len() {
        sub_proof_request_builder.add_revealed_attr(&attributes[i]).unwrap();
      }
      let sub_proof_request = sub_proof_request_builder.finalize().unwrap();

      return sub_proof_request;
    }

    pub fn verify_proof(
      presented_proof: ProofPresentation,
      proof_request: ProofRequest,
      credential_definitions: &HashMap<String, CredentialDefinition>,
      credential_schemas: &HashMap<String, CredentialSchema>
    ) {



      let mut proof_verifier = CryptoVerifier::new_proof_verifier().unwrap();

      let mut non_credential_schema_builder = CryptoIssuer::new_non_credential_schema_builder().unwrap();
      non_credential_schema_builder.add_attr("master_secret").unwrap();
      let non_credential_schema = non_credential_schema_builder.finalize().unwrap();

      let mut pub_key;
      let mut credential_schema_builder;
      let mut sub_proof_request_builder;
      for sub_proof_request in proof_request.sub_proof_requests {
        credential_schema_builder = CryptoIssuer::new_credential_schema_builder().unwrap();
        for property in credential_schemas.get(&sub_proof_request.schema).unwrap().properties.keys() {
          credential_schema_builder.add_attr(property).unwrap();
        }

        sub_proof_request_builder = CryptoVerifier::new_sub_proof_request_builder().unwrap();
        for property in sub_proof_request.revealed_attributes {
          sub_proof_request_builder.add_revealed_attr(&property).unwrap();
        }

        pub_key = &credential_definitions.get(&sub_proof_request.schema).unwrap().public_key;
        proof_verifier.add_sub_proof_request(
          &sub_proof_request_builder.finalize().unwrap(),
          &credential_schema_builder.finalize().unwrap(),
          &non_credential_schema,
          &pub_key,
          None,
          None
        ).unwrap();
      }

      // Create Ursa proof object
      let mut sub_proofs: Vec<SubProof> = Vec::new();
      for vc in presented_proof.verifiable_credential {
        sub_proofs.push(serde_json::from_str(&vc.proof.proof).unwrap());
      }
      let serialized = format!(r###"
        proofs: {},
        aggregated_proof: {}"###,
        serde_json::to_string(&sub_proofs).unwrap(),
        &presented_proof.proof.aggregated_proof
      );
      let ursa_proof: CryptoProof = serde_json::from_str(&serialized).unwrap();

      assert!(proof_verifier.verify(&ursa_proof, &presented_proof.proof.nonce).unwrap());
    }

    /**
     * Decoding BigNumber representations to raw values.
     * Indy currently does not offer a standard for this, everyone is free and obliged to implement that themselves
     * See: https://jira.hyperledger.org/browse/IS-786
     */
    // TODO: BigNumbers will lead to problems when working with predicates, since they only accept i32 values
    fn decode_value(&self, encoded: &str) -> String{
      let val = BigNumber::from_dec(encoded).unwrap();
      let bytes = BigNumber::to_bytes(&val).unwrap();
      let decoded = String::from_utf8(bytes).unwrap();
      return decoded;
    }

  }

}
