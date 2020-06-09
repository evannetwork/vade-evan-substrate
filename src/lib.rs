extern crate ursa;
extern crate secp256k1;
extern crate sha3;
extern crate hex;
#[macro_use]
pub extern crate log;

extern crate vade;
extern crate uuid;

pub mod application;
pub mod crypto;

#[macro_use]
pub mod utils;
pub mod resolver;

pub mod wasm_lib;

use async_trait::async_trait;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use vade::{
    Vade,
    traits::MessageConsumer,
};
use ursa::cl::Witness;
use crate::{
    application::issuer::Issuer,
    application::prover::Prover,
    application::verifier::Verifier,
    application::datatypes::{
        Credential,
        CredentialDefinition,
        CredentialOffer,
        CredentialPrivateKey,
        CredentialProposal,
        CredentialRequest,
        CredentialSchema,
        CredentialSecretsBlindingFactors,
        MasterSecret,
        ProofPresentation,
        ProofRequest,
        ProofVerification,
        RevocationKeyPrivate,
        RevocationRegistryDefinition,
        SchemaProperty,
        SubProofRequest,
        RevocationIdInformation,
        RevocationState
    },
};
use simple_error::SimpleError;

const EXAMPLE_CREDENTIAL_SCHEMA: &str = r###"
{
    "id": "did:evan:zkp:0x123451234512345123451234512346",
    "type": "EvanVCSchema",
    "name": "test_schema",
    "author": "did:evan:testcore:0x0F737D1478eA29df0856169F25cA9129035d6FD1",
    "createdAt": "2020-05-19T12:54:55.000Z",
    "description": "Test description",
    "properties": {
        "test_property_string": {
            "type": "string"
        }
    },
    "required": [
        "test_property_string"
    ],
    "additionalProperties": false,
    "proof": {
        "type": "EcdsaPublicKeySecp256k1",
        "created": "2020-05-19T12:54:55.000Z",
        "proofPurpose": "assertionMethod",
        "verificationMethod": "null",
        "jws": "eyJ0eXAiOiJKV1QiLCJhbGciOiJFUzI1NkstUiJ9.eyJpYXQiOiIyMDIwLTA1LTE5VDEyOjU0OjU1LjAwMFoiLCJkb2MiOnsiaWQiOiJkaWQ6ZXZhbjp6a3A6MHgxMjM0NTEyMzQ1MTIzNDUxMjM0NTEyMzQ1MTIzNDUiLCJ0eXBlIjoiRXZhblZDU2NoZW1hIiwibmFtZSI6InRlc3Rfc2NoZW1hIiwiYXV0aG9yIjoiZGlkOmV2YW46dGVzdGNvcmU6MHgwRjczN0QxNDc4ZUEyOWRmMDg1NjE2OUYyNWNBOTEyOTAzNWQ2RkQxIiwiY3JlYXRlZEF0IjoiMjAyMC0wNS0xOVQxMjo1NDo1NS4wMDBaIiwiZGVzY3JpcHRpb24iOiJUZXN0IGRlc2NyaXB0aW9uIiwicHJvcGVydGllcyI6eyJ0ZXN0X3Byb3BlcnR5X3N0cmluZyI6eyJ0eXBlIjoic3RyaW5nIn19LCJyZXF1aXJlZCI6WyJ0ZXN0X3Byb3BlcnR5X3N0cmluZyJdLCJhZGRpdGlvbmFsUHJvcGVydGllcyI6ZmFsc2V9LCJpc3MiOiJkaWQ6ZXZhbjp0ZXN0Y29yZToweDBGNzM3RDE0NzhlQTI5ZGYwODU2MTY5RjI1Y0E5MTI5MDM1ZDZGRDEifQ.byfS5tIbnCN1M4PtfQQ9mq9mR2pIzgmBFoFNrGkINJBDVxPmKC2S337a2ulytG0G9upyAuOWVMBXESxQdF_MjwA"
    }
}
"###;
const EXAMPLE_CREDENTIAL_DEFINITION: &str = r###"{"id":"did:evan:testcore:0x0F737D1478eA29df0856169F25cA9129035d6FD2","type":"EvanZKPCredentialDefinition","issuer":"did:evan:testcore:0x0F737D1478eA29df0856169F25cA9129035d6FD1","schema":"did:evan:zkp:0x123451234512345123451234512345","createdAt":"2020-05-27T11:23:01.000Z","publicKey":{"p_key":{"n":"27131340063939322891656831596176230199824716996663903821692752356948790981221","s":"24601132371931130435043973895077576734435897938934709578046149583594146369014","r":{"test_property_string":"21429234831755438847456330971606273774402595834684217454680612348643663469912","master_secret":"16877484343966108841178447530050489123536322352202902799631481910760277064645"},"rctxt":"3939746068852726286822409423588059185471986249895839173845735084177148374989","z":"18849699567445135156965624823476403961290685608806045968189130474843062756119"},"r_key":{"g":"1 141EE095BA90AA65AFDD38F4664468659BE380D9F4E584E9ACF73B39EA8D0693 1 0C0E1787D897E0CBDB53BF34C104FDEEE95AD9B0458E15A6245EB5A422812CF2 2 095E45DDF417D05FB10933FFC63D474548B7FFFF7888802F07FFFFFF7D07A8A8","g_dash":"1 1308168BBF3F7DDD586E17451A5FAA7135CB8F1797F9D514DA09D4542CB3476B 1 1188AA61E4C8CC53880AEE9DFC456231CFD4F96143067A88A6F431FF89BA901E 1 06FE70A0341D258EA77DB99F0A8221E29B521C0828CF63D9DADC1F815FCF10B7 1 1979E4DF2CA7A1F19C1011077480A14AA8C2DB00E8628CCEDEC774DCE1245EA1 2 095E45DDF417D05FB10933FFC63D474548B7FFFF7888802F07FFFFFF7D07A8A8 1 0000000000000000000000000000000000000000000000000000000000000000","h":"1 072E6091F6AC976098F7F3502E5416903C38EC356DD5494D6108F56389BFA36D 1 152AE73EF413DF727BBC25B61D55EAD528C9C3A895BE19763D008BBE85274A7F 2 095E45DDF417D05FB10933FFC63D474548B7FFFF7888802F07FFFFFF7D07A8A8","h0":"1 1259475BE4D53507A93D517E356436D94B2C4D2CAD2741BFAE4FE9DEEEFFC37B 1 01BFADD7AEDC8640A4BC84AEF02D3C3019F9B1B56EBDCB159F68FFA6736F7E97 2 095E45DDF417D05FB10933FFC63D474548B7FFFF7888802F07FFFFFF7D07A8A8","h1":"1 04FD6E94B3F0F692D297A28900B6A5BBACA33858F90DD7743C20F8983503727C 1 1FF8EC8DD38E7C9915A2C22D0FFA3FE1458BE778E763365378DE909D6D697B7D 2 095E45DDF417D05FB10933FFC63D474548B7FFFF7888802F07FFFFFF7D07A8A8","h2":"1 0135D5B61C44D3624CF93DDFF99905B971B9047D0F19289D85C8D742595A3223 1 19004A1A5EB47E6728F0E5CE700C4B222CB1FD711BAD50E2C3CB6C21218AA8FA 2 095E45DDF417D05FB10933FFC63D474548B7FFFF7888802F07FFFFFF7D07A8A8","htilde":"1 2438F59828648CC663D6AA8B4F5B5B25B2E715FFD910D1A20B2026C9B6CB90DC 1 213AEBBC04791A7E4EFF4E3F4E4E7D323149E8290EB29824BC47E761DB601248 2 095E45DDF417D05FB10933FFC63D474548B7FFFF7888802F07FFFFFF7D07A8A8","h_cap":"1 196A247F801103275D2D516958F2FBE0F2842E1E0A5E7221247CE36580B62F48 1 115194FD3939F8A0C52C98D63F5819433871A1EA5A6465C81AF0C82F8242B24C 1 1458A0ADAE42D50798C59EAC7096813A77CF631F5BBAED3B7DB76AFD01F78E99 1 02217DB60147EE60152A9B1DCA1BC63E08562EC23C07ADC9C6AB384B83EA4B3F 2 095E45DDF417D05FB10933FFC63D474548B7FFFF7888802F07FFFFFF7D07A8A8 1 0000000000000000000000000000000000000000000000000000000000000000","u":"1 03A3801A32A8AF56BE9F748FD697E19C616B6533CD376BFC6CDFFF021A78D7E9 1 251C122EC97016FB5E40C7D1FF4A5D950674E52D5C13E32A084BE91F1989C3AD 1 1F6F9D35A8535C2B60D92F36DADD6780DD6D7BDD58F3B569436153ECEF10206E 1 16C3D4A0227B26EFD657E7720A066D47772A104C74FB577378721EEA8AD4C3E2 2 095E45DDF417D05FB10933FFC63D474548B7FFFF7888802F07FFFFFF7D07A8A8 1 0000000000000000000000000000000000000000000000000000000000000000","pk":"1 1579071428AB985A3FDD8FB8F2C1363057C3EBB091D3A26FB8C470D17B8D6FA6 1 09666F60B3D0B7EADEA9C12EA63F0607FD69C0F7CF068641EB2B3720FABB09A3 2 095E45DDF417D05FB10933FFC63D474548B7FFFF7888802F07FFFFFF7D07A8A8","y":"1 003568F6B1C96DD7D7B80005ACF973E58805503E9C3AE2FC00C3BA6674ACDE5C 1 198DF62DC9C35D8D456337B5ABC939BD304901ED058F640D81965F8A20D18EA4 1 0EB25468961BCA5617530B36F0CD6AB2FA63CD62C7C3F474824E58ECA08CC4B5 1 06AF443D14A3DEB73898AF44945F4977B273212C8E54FB6001D86D35766A2DDB 2 095E45DDF417D05FB10933FFC63D474548B7FFFF7888802F07FFFFFF7D07A8A8 1 0000000000000000000000000000000000000000000000000000000000000000"}},"publicKeyCorrectnessProof":{"c":"90388047487196786029273725708526536579308051208531905395918591824466015689172","xz_cap":"328171710785770615959149537207261096440116173820708109700992206345404124835028577890426694671123957031066450350300719236916181037502625064217014785296858","xr_cap":[["test_property_string","197166837281265867075437010936813507587898318284379650979777759364042229136366809156829701205516523340197407987697853244106528497018678349751492483475657"],["master_secret","527954057021137727821259553322160502467843701572495543529976846774718751843883607225781154690436468553366756362109741337225248637917541792272497738023540"]]},"proof":{"type":"EcdsaPublicKeySecp256k1","created":"2020-05-27T11:23:07.000Z","proofPurpose":"assertionMethod","verificationMethod":"did:evan:testcore:0x0f737d1478ea29df0856169f25ca9129035d6fd1#key-1","jws":"eyJ0eXAiOiJKV1QiLCJhbGciOiJFUzI1NkstUiJ9.eyJpYXQiOiIyMDIwLTA1LTI3VDExOjIzOjA3LjAwMFoiLCJkb2MiOnsiaWQiOiJkaWQ6ZXZhbjp0ZXN0Y29yZToweDBGNzM3RDE0NzhlQTI5ZGYwODU2MTY5RjI1Y0E5MTI5MDM1ZDZGRDIiLCJ0eXBlIjoiRXZhblpLUENyZWRlbnRpYWxEZWZpbml0aW9uIiwiaXNzdWVyIjoiZGlkOmV2YW46dGVzdGNvcmU6MHgwRjczN0QxNDc4ZUEyOWRmMDg1NjE2OUYyNWNBOTEyOTAzNWQ2RkQxIiwic2NoZW1hIjoiZGlkOmV2YW46emtwOjB4MTIzNDUxMjM0NTEyMzQ1MTIzNDUxMjM0NTEyMzQ1IiwiY3JlYXRlZEF0IjoiMjAyMC0wNS0yN1QxMToyMzowMS4wMDBaIiwicHVibGljS2V5Ijp7InBfa2V5Ijp7Im4iOiIyNzEzMTM0MDA2MzkzOTMyMjg5MTY1NjgzMTU5NjE3NjIzMDE5OTgyNDcxNjk5NjY2MzkwMzgyMTY5Mjc1MjM1Njk0ODc5MDk4MTIyMSIsInMiOiIyNDYwMTEzMjM3MTkzMTEzMDQzNTA0Mzk3Mzg5NTA3NzU3NjczNDQzNTg5NzkzODkzNDcwOTU3ODA0NjE0OTU4MzU5NDE0NjM2OTAxNCIsInIiOnsidGVzdF9wcm9wZXJ0eV9zdHJpbmciOiIyMTQyOTIzNDgzMTc1NTQzODg0NzQ1NjMzMDk3MTYwNjI3Mzc3NDQwMjU5NTgzNDY4NDIxNzQ1NDY4MDYxMjM0ODY0MzY2MzQ2OTkxMiIsIm1hc3Rlcl9zZWNyZXQiOiIxNjg3NzQ4NDM0Mzk2NjEwODg0MTE3ODQ0NzUzMDA1MDQ4OTEyMzUzNjMyMjM1MjIwMjkwMjc5OTYzMTQ4MTkxMDc2MDI3NzA2NDY0NSJ9LCJyY3R4dCI6IjM5Mzk3NDYwNjg4NTI3MjYyODY4MjI0MDk0MjM1ODgwNTkxODU0NzE5ODYyNDk4OTU4MzkxNzM4NDU3MzUwODQxNzcxNDgzNzQ5ODkiLCJ6IjoiMTg4NDk2OTk1Njc0NDUxMzUxNTY5NjU2MjQ4MjM0NzY0MDM5NjEyOTA2ODU2MDg4MDYwNDU5NjgxODkxMzA0NzQ4NDMwNjI3NTYxMTkifSwicl9rZXkiOnsiZyI6IjEgMTQxRUUwOTVCQTkwQUE2NUFGREQzOEY0NjY0NDY4NjU5QkUzODBEOUY0RTU4NEU5QUNGNzNCMzlFQThEMDY5MyAxIDBDMEUxNzg3RDg5N0UwQ0JEQjUzQkYzNEMxMDRGREVFRTk1QUQ5QjA0NThFMTVBNjI0NUVCNUE0MjI4MTJDRjIgMiAwOTVFNDVEREY0MTdEMDVGQjEwOTMzRkZDNjNENDc0NTQ4QjdGRkZGNzg4ODgwMkYwN0ZGRkZGRjdEMDdBOEE4IiwiZ19kYXNoIjoiMSAxMzA4MTY4QkJGM0Y3RERENTg2RTE3NDUxQTVGQUE3MTM1Q0I4RjE3OTdGOUQ1MTREQTA5RDQ1NDJDQjM0NzZCIDEgMTE4OEFBNjFFNEM4Q0M1Mzg4MEFFRTlERkM0NTYyMzFDRkQ0Rjk2MTQzMDY3QTg4QTZGNDMxRkY4OUJBOTAxRSAxIDA2RkU3MEEwMzQxRDI1OEVBNzdEQjk5RjBBODIyMUUyOUI1MjFDMDgyOENGNjNEOURBREMxRjgxNUZDRjEwQjcgMSAxOTc5RTRERjJDQTdBMUYxOUMxMDExMDc3NDgwQTE0QUE4QzJEQjAwRTg2MjhDQ0VERUM3NzREQ0UxMjQ1RUExIDIgMDk1RTQ1RERGNDE3RDA1RkIxMDkzM0ZGQzYzRDQ3NDU0OEI3RkZGRjc4ODg4MDJGMDdGRkZGRkY3RDA3QThBOCAxIDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAiLCJoIjoiMSAwNzJFNjA5MUY2QUM5NzYwOThGN0YzNTAyRTU0MTY5MDNDMzhFQzM1NkRENTQ5NEQ2MTA4RjU2Mzg5QkZBMzZEIDEgMTUyQUU3M0VGNDEzREY3MjdCQkMyNUI2MUQ1NUVBRDUyOEM5QzNBODk1QkUxOTc2M0QwMDhCQkU4NTI3NEE3RiAyIDA5NUU0NURERjQxN0QwNUZCMTA5MzNGRkM2M0Q0NzQ1NDhCN0ZGRkY3ODg4ODAyRjA3RkZGRkZGN0QwN0E4QTgiLCJoMCI6IjEgMTI1OTQ3NUJFNEQ1MzUwN0E5M0Q1MTdFMzU2NDM2RDk0QjJDNEQyQ0FEMjc0MUJGQUU0RkU5REVFRUZGQzM3QiAxIDAxQkZBREQ3QUVEQzg2NDBBNEJDODRBRUYwMkQzQzMwMTlGOUIxQjU2RUJEQ0IxNTlGNjhGRkE2NzM2RjdFOTcgMiAwOTVFNDVEREY0MTdEMDVGQjEwOTMzRkZDNjNENDc0NTQ4QjdGRkZGNzg4ODgwMkYwN0ZGRkZGRjdEMDdBOEE4IiwiaDEiOiIxIDA0RkQ2RTk0QjNGMEY2OTJEMjk3QTI4OTAwQjZBNUJCQUNBMzM4NThGOTBERDc3NDNDMjBGODk4MzUwMzcyN0MgMSAxRkY4RUM4REQzOEU3Qzk5MTVBMkMyMkQwRkZBM0ZFMTQ1OEJFNzc4RTc2MzM2NTM3OERFOTA5RDZENjk3QjdEIDIgMDk1RTQ1RERGNDE3RDA1RkIxMDkzM0ZGQzYzRDQ3NDU0OEI3RkZGRjc4ODg4MDJGMDdGRkZGRkY3RDA3QThBOCIsImgyIjoiMSAwMTM1RDVCNjFDNDREMzYyNENGOTNEREZGOTk5MDVCOTcxQjkwNDdEMEYxOTI4OUQ4NUM4RDc0MjU5NUEzMjIzIDEgMTkwMDRBMUE1RUI0N0U2NzI4RjBFNUNFNzAwQzRCMjIyQ0IxRkQ3MTFCQUQ1MEUyQzNDQjZDMjEyMThBQThGQSAyIDA5NUU0NURERjQxN0QwNUZCMTA5MzNGRkM2M0Q0NzQ1NDhCN0ZGRkY3ODg4ODAyRjA3RkZGRkZGN0QwN0E4QTgiLCJodGlsZGUiOiIxIDI0MzhGNTk4Mjg2NDhDQzY2M0Q2QUE4QjRGNUI1QjI1QjJFNzE1RkZEOTEwRDFBMjBCMjAyNkM5QjZDQjkwREMgMSAyMTNBRUJCQzA0NzkxQTdFNEVGRjRFM0Y0RTRFN0QzMjMxNDlFODI5MEVCMjk4MjRCQzQ3RTc2MURCNjAxMjQ4IDIgMDk1RTQ1RERGNDE3RDA1RkIxMDkzM0ZGQzYzRDQ3NDU0OEI3RkZGRjc4ODg4MDJGMDdGRkZGRkY3RDA3QThBOCIsImhfY2FwIjoiMSAxOTZBMjQ3RjgwMTEwMzI3NUQyRDUxNjk1OEYyRkJFMEYyODQyRTFFMEE1RTcyMjEyNDdDRTM2NTgwQjYyRjQ4IDEgMTE1MTk0RkQzOTM5RjhBMEM1MkM5OEQ2M0Y1ODE5NDMzODcxQTFFQTVBNjQ2NUM4MUFGMEM4MkY4MjQyQjI0QyAxIDE0NThBMEFEQUU0MkQ1MDc5OEM1OUVBQzcwOTY4MTNBNzdDRjYzMUY1QkJBRUQzQjdEQjc2QUZEMDFGNzhFOTkgMSAwMjIxN0RCNjAxNDdFRTYwMTUyQTlCMURDQTFCQzYzRTA4NTYyRUMyM0MwN0FEQzlDNkFCMzg0QjgzRUE0QjNGIDIgMDk1RTQ1RERGNDE3RDA1RkIxMDkzM0ZGQzYzRDQ3NDU0OEI3RkZGRjc4ODg4MDJGMDdGRkZGRkY3RDA3QThBOCAxIDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAiLCJ1IjoiMSAwM0EzODAxQTMyQThBRjU2QkU5Rjc0OEZENjk3RTE5QzYxNkI2NTMzQ0QzNzZCRkM2Q0RGRkYwMjFBNzhEN0U5IDEgMjUxQzEyMkVDOTcwMTZGQjVFNDBDN0QxRkY0QTVEOTUwNjc0RTUyRDVDMTNFMzJBMDg0QkU5MUYxOTg5QzNBRCAxIDFGNkY5RDM1QTg1MzVDMkI2MEQ5MkYzNkRBREQ2NzgwREQ2RDdCREQ1OEYzQjU2OTQzNjE1M0VDRUYxMDIwNkUgMSAxNkMzRDRBMDIyN0IyNkVGRDY1N0U3NzIwQTA2NkQ0Nzc3MkExMDRDNzRGQjU3NzM3ODcyMUVFQThBRDRDM0UyIDIgMDk1RTQ1RERGNDE3RDA1RkIxMDkzM0ZGQzYzRDQ3NDU0OEI3RkZGRjc4ODg4MDJGMDdGRkZGRkY3RDA3QThBOCAxIDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAiLCJwayI6IjEgMTU3OTA3MTQyOEFCOTg1QTNGREQ4RkI4RjJDMTM2MzA1N0MzRUJCMDkxRDNBMjZGQjhDNDcwRDE3QjhENkZBNiAxIDA5NjY2RjYwQjNEMEI3RUFERUE5QzEyRUE2M0YwNjA3RkQ2OUMwRjdDRjA2ODY0MUVCMkIzNzIwRkFCQjA5QTMgMiAwOTVFNDVEREY0MTdEMDVGQjEwOTMzRkZDNjNENDc0NTQ4QjdGRkZGNzg4ODgwMkYwN0ZGRkZGRjdEMDdBOEE4IiwieSI6IjEgMDAzNTY4RjZCMUM5NkREN0Q3QjgwMDA1QUNGOTczRTU4ODA1NTAzRTlDM0FFMkZDMDBDM0JBNjY3NEFDREU1QyAxIDE5OERGNjJEQzlDMzVEOEQ0NTYzMzdCNUFCQzkzOUJEMzA0OTAxRUQwNThGNjQwRDgxOTY1RjhBMjBEMThFQTQgMSAwRUIyNTQ2ODk2MUJDQTU2MTc1MzBCMzZGMENENkFCMkZBNjNDRDYyQzdDM0Y0NzQ4MjRFNThFQ0EwOENDNEI1IDEgMDZBRjQ0M0QxNEEzREVCNzM4OThBRjQ0OTQ1RjQ5NzdCMjczMjEyQzhFNTRGQjYwMDFEODZEMzU3NjZBMkREQiAyIDA5NUU0NURERjQxN0QwNUZCMTA5MzNGRkM2M0Q0NzQ1NDhCN0ZGRkY3ODg4ODAyRjA3RkZGRkZGN0QwN0E4QTggMSAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwIn19LCJwdWJsaWNLZXlDb3JyZWN0bmVzc1Byb29mIjp7ImMiOiI5MDM4ODA0NzQ4NzE5Njc4NjAyOTI3MzcyNTcwODUyNjUzNjU3OTMwODA1MTIwODUzMTkwNTM5NTkxODU5MTgyNDQ2NjAxNTY4OTE3MiIsInh6X2NhcCI6IjMyODE3MTcxMDc4NTc3MDYxNTk1OTE0OTUzNzIwNzI2MTA5NjQ0MDExNjE3MzgyMDcwODEwOTcwMDk5MjIwNjM0NTQwNDEyNDgzNTAyODU3Nzg5MDQyNjY5NDY3MTEyMzk1NzAzMTA2NjQ1MDM1MDMwMDcxOTIzNjkxNjE4MTAzNzUwMjYyNTA2NDIxNzAxNDc4NTI5Njg1OCIsInhyX2NhcCI6W1sidGVzdF9wcm9wZXJ0eV9zdHJpbmciLCIxOTcxNjY4MzcyODEyNjU4NjcwNzU0MzcwMTA5MzY4MTM1MDc1ODc4OTgzMTgyODQzNzk2NTA5Nzk3Nzc3NTkzNjQwNDIyMjkxMzYzNjY4MDkxNTY4Mjk3MDEyMDU1MTY1MjMzNDAxOTc0MDc5ODc2OTc4NTMyNDQxMDY1Mjg0OTcwMTg2NzgzNDk3NTE0OTI0ODM0NzU2NTciXSxbIm1hc3Rlcl9zZWNyZXQiLCI1Mjc5NTQwNTcwMjExMzc3Mjc4MjEyNTk1NTMzMjIxNjA1MDI0Njc4NDM3MDE1NzI0OTU1NDM1Mjk5NzY4NDY3NzQ3MTg3NTE4NDM4ODM2MDcyMjU3ODExNTQ2OTA0MzY0Njg1NTMzNjY3NTYzNjIxMDk3NDEzMzcyMjUyNDg2Mzc5MTc1NDE3OTIyNzI0OTc3MzgwMjM1NDAiXV19fSwiaXNzIjoiZGlkOmV2YW46dGVzdGNvcmU6MHgwRjczN0QxNDc4ZUEyOWRmMDg1NjE2OUYyNWNBOTEyOTAzNWQ2RkQxIn0.PSB2q5DuebDD_VkiO321NKDBT_K7Av4drWJVPwmTAeFpweJIMHLZcFvMcM7NtPBwGDwf7lrT9dwGuAjc5e-ixAA"}}"###;
const EXAMPLE_REVOCATION_REGISTRY_DEFINITION: &str = r###"{"id":"did:evan:testcore:0x0F737D1478eA29df0856169F25cA9129035d6FD2","credentialDefinition":"did:evan:testcore:0x0F737D1478eA29df0856169F25cA9129035d6FD2","updatedAt":"2020-05-28T07:52:19.000Z","registry":{"accum":"21 132D0FAD6E78BF66C6CE8F41527E878F259E02F5BEF564E5A08C42C64D669D78E 21 1315F4CD5AA9B18F1EF188C570C17861E7F581BAF60A5C29BE174F6786DD71D9E 6 5C5C8CFBFC0899244ECF9C7C8C395BB092C1DA5B55ABDAE23EC9888EBAB8A0EA 4 2F4866B33810F0E45432DF7EEAD4B6C30F5201338D6DFA286B99B0F97D453207 6 793A77FC3983325265825A160234DDBEFEBC9DDEC3EC8EBEA242D6F516DE2137 4 2814CD5A23160756CD328DA7E240F2122816BFE8938201DA5BB821BA9C441FA0"},"registryDelta":{"accum":"21 132D0FAD6E78BF66C6CE8F41527E878F259E02F5BEF564E5A08C42C64D669D78E 21 1315F4CD5AA9B18F1EF188C570C17861E7F581BAF60A5C29BE174F6786DD71D9E 6 5C5C8CFBFC0899244ECF9C7C8C395BB092C1DA5B55ABDAE23EC9888EBAB8A0EA 4 2F4866B33810F0E45432DF7EEAD4B6C30F5201338D6DFA286B99B0F97D453207 6 793A77FC3983325265825A160234DDBEFEBC9DDEC3EC8EBEA242D6F516DE2137 4 2814CD5A23160756CD328DA7E240F2122816BFE8938201DA5BB821BA9C441FA0"},"tails":{"size":85,"current_index":0,"g_dash":"1 1308168BBF3F7DDD586E17451A5FAA7135CB8F1797F9D514DA09D4542CB3476B 1 1188AA61E4C8CC53880AEE9DFC456231CFD4F96143067A88A6F431FF89BA901E 1 06FE70A0341D258EA77DB99F0A8221E29B521C0828CF63D9DADC1F815FCF10B7 1 1979E4DF2CA7A1F19C1011077480A14AA8C2DB00E8628CCEDEC774DCE1245EA1 2 095E45DDF417D05FB10933FFC63D474548B7FFFF7888802F07FFFFFF7D07A8A8 1 0000000000000000000000000000000000000000000000000000000000000000","gamma":"078EBCFD7AED245E63521DFB33A2073D109496487706FBDCE57FDAA6F01DDD24"},"revocationPublicKey":{"z":"1 2296B91BA5D774500DD3521CD1834321D7D546F30A774CA836EE4847D5FFA85A 1 2360672045D77A59A2919C98C4E7D0458A84F4B33F3FCC6F20F8EB5C5589F262 1 1A048E6181847D91679044AD0A7B9409B9CE195E0679A55183EEC6204921907E 1 1671768909C1F45340E94F95AD3F7FFCE02C65BF6EB7C9D666486E2819093AE2 1 0CD5A098CBFFD3B1608BBBD37E2E259E2935E158B37189B32CE354109CBFFF8D 1 1BEF8EFF89F2315AA34B66AEE2DC011ADF3AFB14460A02B38B3CDF15C3058538 1 028B8745A491DEDBD188AADB660C52982F75AEBB1C6F7B67073820BF746CCCFF 1 1BB3B1D3FC181DE1EE604FEEC63C1B24AB835A09A9021795C2EC01BA44C40890 1 025F736CC7AE66F55F278EFEBD96B9D9BA49E756B3DD8B20892A92E088779C7D 1 1C421BC870B2125EE38570B9DA9708AB568B28E6E8A210FE72BC3E693023803B 1 244DEF48B8A6131D3EF4DDA62141DDE1AD49475D71F6588624D01FC635A2129D 1 1D91652430155B61A98A753B6F6AAE24C28591719C7A834A874F68D11B660258"},"maximumCredentialCount":42,"proof":{"type":"EcdsaPublicKeySecp256k1","created":"2020-05-28T07:52:19.000Z","proofPurpose":"assertionMethod","verificationMethod":"did:evan:testcore:0x0f737d1478ea29df0856169f25ca9129035d6fd1#key-1","jws":"eyJ0eXAiOiJKV1QiLCJhbGciOiJFUzI1NkstUiJ9.eyJpYXQiOiIyMDIwLTA1LTI4VDA3OjUyOjE5LjAwMFoiLCJkb2MiOnsiaWQiOiJkaWQ6ZXZhbjp0ZXN0Y29yZToweDBGNzM3RDE0NzhlQTI5ZGYwODU2MTY5RjI1Y0E5MTI5MDM1ZDZGRDIiLCJjcmVkZW50aWFsRGVmaW5pdGlvbiI6ImRpZDpldmFuOnRlc3Rjb3JlOjB4MEY3MzdEMTQ3OGVBMjlkZjA4NTYxNjlGMjVjQTkxMjkwMzVkNkZEMiIsInVwZGF0ZWRBdCI6IjIwMjAtMDUtMjhUMDc6NTI6MTkuMDAwWiIsInJlZ2lzdHJ5Ijp7ImFjY3VtIjoiMjEgMTMyRDBGQUQ2RTc4QkY2NkM2Q0U4RjQxNTI3RTg3OEYyNTlFMDJGNUJFRjU2NEU1QTA4QzQyQzY0RDY2OUQ3OEUgMjEgMTMxNUY0Q0Q1QUE5QjE4RjFFRjE4OEM1NzBDMTc4NjFFN0Y1ODFCQUY2MEE1QzI5QkUxNzRGNjc4NkRENzFEOUUgNiA1QzVDOENGQkZDMDg5OTI0NEVDRjlDN0M4QzM5NUJCMDkyQzFEQTVCNTVBQkRBRTIzRUM5ODg4RUJBQjhBMEVBIDQgMkY0ODY2QjMzODEwRjBFNDU0MzJERjdFRUFENEI2QzMwRjUyMDEzMzhENkRGQTI4NkI5OUIwRjk3RDQ1MzIwNyA2IDc5M0E3N0ZDMzk4MzMyNTI2NTgyNUExNjAyMzREREJFRkVCQzlEREVDM0VDOEVCRUEyNDJENkY1MTZERTIxMzcgNCAyODE0Q0Q1QTIzMTYwNzU2Q0QzMjhEQTdFMjQwRjIxMjI4MTZCRkU4OTM4MjAxREE1QkI4MjFCQTlDNDQxRkEwIn0sInJlZ2lzdHJ5RGVsdGEiOnsiYWNjdW0iOiIyMSAxMzJEMEZBRDZFNzhCRjY2QzZDRThGNDE1MjdFODc4RjI1OUUwMkY1QkVGNTY0RTVBMDhDNDJDNjRENjY5RDc4RSAyMSAxMzE1RjRDRDVBQTlCMThGMUVGMTg4QzU3MEMxNzg2MUU3RjU4MUJBRjYwQTVDMjlCRTE3NEY2Nzg2REQ3MUQ5RSA2IDVDNUM4Q0ZCRkMwODk5MjQ0RUNGOUM3QzhDMzk1QkIwOTJDMURBNUI1NUFCREFFMjNFQzk4ODhFQkFCOEEwRUEgNCAyRjQ4NjZCMzM4MTBGMEU0NTQzMkRGN0VFQUQ0QjZDMzBGNTIwMTMzOEQ2REZBMjg2Qjk5QjBGOTdENDUzMjA3IDYgNzkzQTc3RkMzOTgzMzI1MjY1ODI1QTE2MDIzNEREQkVGRUJDOURERUMzRUM4RUJFQTI0MkQ2RjUxNkRFMjEzNyA0IDI4MTRDRDVBMjMxNjA3NTZDRDMyOERBN0UyNDBGMjEyMjgxNkJGRTg5MzgyMDFEQTVCQjgyMUJBOUM0NDFGQTAifSwidGFpbHMiOnsic2l6ZSI6ODUsImN1cnJlbnRfaW5kZXgiOjAsImdfZGFzaCI6IjEgMTMwODE2OEJCRjNGN0RERDU4NkUxNzQ1MUE1RkFBNzEzNUNCOEYxNzk3RjlENTE0REEwOUQ0NTQyQ0IzNDc2QiAxIDExODhBQTYxRTRDOENDNTM4ODBBRUU5REZDNDU2MjMxQ0ZENEY5NjE0MzA2N0E4OEE2RjQzMUZGODlCQTkwMUUgMSAwNkZFNzBBMDM0MUQyNThFQTc3REI5OUYwQTgyMjFFMjlCNTIxQzA4MjhDRjYzRDlEQURDMUY4MTVGQ0YxMEI3IDEgMTk3OUU0REYyQ0E3QTFGMTlDMTAxMTA3NzQ4MEExNEFBOEMyREIwMEU4NjI4Q0NFREVDNzc0RENFMTI0NUVBMSAyIDA5NUU0NURERjQxN0QwNUZCMTA5MzNGRkM2M0Q0NzQ1NDhCN0ZGRkY3ODg4ODAyRjA3RkZGRkZGN0QwN0E4QTggMSAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwIiwiZ2FtbWEiOiIwNzhFQkNGRDdBRUQyNDVFNjM1MjFERkIzM0EyMDczRDEwOTQ5NjQ4NzcwNkZCRENFNTdGREFBNkYwMURERDI0In0sInJldm9jYXRpb25QdWJsaWNLZXkiOnsieiI6IjEgMjI5NkI5MUJBNUQ3NzQ1MDBERDM1MjFDRDE4MzQzMjFEN0Q1NDZGMzBBNzc0Q0E4MzZFRTQ4NDdENUZGQTg1QSAxIDIzNjA2NzIwNDVENzdBNTlBMjkxOUM5OEM0RTdEMDQ1OEE4NEY0QjMzRjNGQ0M2RjIwRjhFQjVDNTU4OUYyNjIgMSAxQTA0OEU2MTgxODQ3RDkxNjc5MDQ0QUQwQTdCOTQwOUI5Q0UxOTVFMDY3OUE1NTE4M0VFQzYyMDQ5MjE5MDdFIDEgMTY3MTc2ODkwOUMxRjQ1MzQwRTk0Rjk1QUQzRjdGRkNFMDJDNjVCRjZFQjdDOUQ2NjY0ODZFMjgxOTA5M0FFMiAxIDBDRDVBMDk4Q0JGRkQzQjE2MDhCQkJEMzdFMkUyNTlFMjkzNUUxNThCMzcxODlCMzJDRTM1NDEwOUNCRkZGOEQgMSAxQkVGOEVGRjg5RjIzMTVBQTM0QjY2QUVFMkRDMDExQURGM0FGQjE0NDYwQTAyQjM4QjNDREYxNUMzMDU4NTM4IDEgMDI4Qjg3NDVBNDkxREVEQkQxODhBQURCNjYwQzUyOTgyRjc1QUVCQjFDNkY3QjY3MDczODIwQkY3NDZDQ0NGRiAxIDFCQjNCMUQzRkMxODFERTFFRTYwNEZFRUM2M0MxQjI0QUI4MzVBMDlBOTAyMTc5NUMyRUMwMUJBNDRDNDA4OTAgMSAwMjVGNzM2Q0M3QUU2NkY1NUYyNzhFRkVCRDk2QjlEOUJBNDlFNzU2QjNERDhCMjA4OTJBOTJFMDg4Nzc5QzdEIDEgMUM0MjFCQzg3MEIyMTI1RUUzODU3MEI5REE5NzA4QUI1NjhCMjhFNkU4QTIxMEZFNzJCQzNFNjkzMDIzODAzQiAxIDI0NERFRjQ4QjhBNjEzMUQzRUY0RERBNjIxNDFEREUxQUQ0OTQ3NUQ3MUY2NTg4NjI0RDAxRkM2MzVBMjEyOUQgMSAxRDkxNjUyNDMwMTU1QjYxQTk4QTc1M0I2RjZBQUUyNEMyODU5MTcxOUM3QTgzNEE4NzRGNjhEMTFCNjYwMjU4In0sIm1heGltdW1DcmVkZW50aWFsQ291bnQiOjQyfSwiaXNzIjoiZGlkOmV2YW46dGVzdGNvcmU6MHgwRjczN0QxNDc4ZUEyOWRmMDg1NjE2OUYyNWNBOTEyOTAzNWQ2RkQxIn0.v9axUE4EoW7R6uOFdowSKgYg_axqwdP6m-hD3glsd2J-xcYcFtIjA4e2NYnPoVWPUMaCCVKld9sbTBV81W_WIQA"}}"###;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCredentialSchemaArguments {
    pub issuer: String,
    pub schema_name: String,
    pub description: String,
    pub properties: HashMap<String, SchemaProperty>,
    pub required_properties: Vec<String>,
    pub allow_additional_properties: bool,
    pub issuer_public_key_did: String,
    pub issuer_proving_key: String,
    pub private_key: String,
    pub identity: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssueCredentialArguments {
    pub issuer: String,
    pub subject: String,
    pub credential_request: CredentialRequest,
    pub credential_revocation_definition: String,
    pub credential_private_key: CredentialPrivateKey,
    pub revocation_private_key: RevocationKeyPrivate,
    pub revocation_information: RevocationIdInformation
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OfferCredentialArguments {
    pub issuer: String,
    pub subject: String,
    pub schema: String,
    pub credential_definition: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresentProofArguments {
    pub proof_request: ProofRequest,
    pub credentials: HashMap<String, Credential>,
    pub witnesses: HashMap<String, Witness>,
    pub master_secret: MasterSecret
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCredentialProposalArguments {
    pub issuer: String,
    pub subject: String,
    pub schema: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestCredentialArguments {
    pub credential_offering: CredentialOffer,
    pub master_secret: MasterSecret,
    pub credential_values: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestProofArguments {
    pub verifier_did: String,
    pub prover_did: String,
    pub sub_proof_requests: Vec<SubProofRequest>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ValidateProofArguments {
    pub presented_proof: ProofPresentation,
    pub proof_request: ProofRequest
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WhitelistIdentityArguments {
  pub private_key: String,
  pub identity: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCredentialDefinitionArguments {
  pub issuer_did: String,
  pub schema_did: String,
  pub issuer_public_key_did: String,
  pub issuer_proving_key: String,
  pub private_key: String,
  pub identity: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateRevocationRegistryDefinitionArguments {
  pub credential_definition: String,
  pub issuer_public_key_did: String,
  pub issuer_proving_key: String,
  pub maximum_credential_count: u32,
  pub private_key: String,
  pub identity: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RevokeCredentialArguments {
  issuer: String,
  revocation_registry_definition: String,
  credential_revocation_id: u32,
  issuer_public_key_did: String,
  issuer_proving_key: String,
  pub private_key: String,
  pub identity: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRevocationRegistryDefinitionResult {
  pub private_key: RevocationKeyPrivate,
  pub revocation_info: RevocationIdInformation,
  pub revocation_registry_definition: RevocationRegistryDefinition
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueCredentialResult {
  pub credential: Credential,
  pub revocation_info: RevocationIdInformation,
  pub revocation_state: RevocationState
}

pub struct VadeTnt {
  vade: Vade
}

impl VadeTnt {
    /// Creates new instance of `VadeTnt`.
    pub fn new(vade: Vade) -> VadeTnt {
        match env_logger::try_init() {
            Ok(_) | Err(_) => (),
        };
        VadeTnt {
          vade
        }
    }
}

#[async_trait(?Send)]
impl MessageConsumer for VadeTnt {
    /// Reacts to `Vade` messages.
    ///
    /// # Arguments
    ///
    /// * `message_data` - arbitrary data for plugin, e.g. a JSON
    async fn handle_message(
        &mut self,
        message_type: &str,
        message_data: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        match message_type {
            "createCredentialDefinition" => self.create_credential_definition(message_data).await,
            "createCredentialOffer" => self.create_credential_offer(message_data).await,
            "createCredentialProposal" => self.create_credential_proposal(message_data).await,
            "createCredentialSchema" => self.create_credential_schema(message_data).await,
            "createRevocationRegistryDefinition" => self.create_revocation_registry_definition(message_data).await,
            "issueCredential" => self.issue_credential(message_data).await,
            "presentProof" => self.present_proof(message_data).await,
            "requestCredential" => self.request_credential(message_data).await,
            "requestProof" => self.request_proof(message_data).await,
            "revokeCredential" => self.revoke_credential(message_data).await,
            "verifyProof" => self.verify_proof(message_data).await,
            "whitelistIdentity" => self.whitelist_identity(message_data).await,
            _ => Err(Box::from(format!("message type '{}' not implemented", message_type)))
        }
    }
}

impl VadeTnt {
    /// Creates a new credential definition and stores it on-chain.
    ///
    /// # Arguments
    /// * `data` - Expects a JSON object representing a `CreateCredentialDefinitionArguments`
    ///
    /// # Returns
    /// * `Option<String>` - The created definition as a JSON object
    async fn create_credential_definition(&mut self, data: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
      let input: CreateCredentialDefinitionArguments = serde_json::from_str(&data)?;
      let schema: CredentialSchema = serde_json::from_str(
          &self.vade.get_did_document(
              &input.schema_did
          ).await?
      ).unwrap();

      let generated_did = self.generate_did(&input.private_key, &input.identity).await?;

      let (definition, pk) = Issuer::create_credential_definition(
        &generated_did,
        &input.issuer_did,
        &schema,
        &input.issuer_public_key_did,
        &input.issuer_proving_key
      );

      let serialized = serde_json::to_string(&(&definition, &pk)).unwrap();
      let serialized_definition = serde_json::to_string(&definition).unwrap();
      self.set_did_document(&generated_did, &serialized_definition, &input.private_key, &input.identity).await?;

      Ok(Some(serialized))
    }

    /// Creates a new credential schema and stores it on-chain.
    ///
    /// # Arguments
    /// * `data` - Expects a JSON object representing a `CreateCredentialSchemaArguments`
    ///
    /// # Returns
    /// * `Option<String>` - The created schema as a JSON object
    async fn create_credential_schema(&mut self, data: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
      let input: CreateCredentialSchemaArguments = serde_json::from_str(&data)?;

      let generated_did = self.generate_did(&input.private_key, &input.identity).await?;

      let schema = Issuer::create_credential_schema(
        &generated_did,
        &input.issuer,
        &input.schema_name,
        &input.description,
        input.properties,
        input.required_properties,
        input.allow_additional_properties,
        &input.issuer_public_key_did,
        &input.issuer_proving_key
      );

      let serialized = serde_json::to_string(&schema).unwrap();
      self.set_did_document(&generated_did, &serialized, &input.private_key, &input.identity).await?;

      Ok(Some(serialized))
    }

    /// Creates a new revocation registry definition and stores it on-chain.
    ///
    /// # Arguments
    /// * `data` - Expects a JSON object representing a `CreateRevocationRegistryDefinitionArguments`
    ///
    /// # Returns
    /// * `Option<String>` - The created revocation registry definition as a JSON object
    async fn create_revocation_registry_definition(&mut self, data: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
      let input: CreateRevocationRegistryDefinitionArguments = serde_json::from_str(&data)?;

      // Resolve credential definition
      let definition: CredentialDefinition = serde_json::from_str(
        &self.vade.get_did_document(
          &input.credential_definition
        ).await?
      ).unwrap();

      let generated_did = self.generate_did(&input.private_key, &input.identity).await?;

      let (definition, private_key, revocation_info) = Issuer::create_revocation_registry_definition(
        &generated_did,
        &definition,
        &input.issuer_public_key_did,
        &input.issuer_proving_key,
        input.maximum_credential_count
      );

      let serialised_def = serde_json::to_string(&definition).unwrap();

      self.set_did_document(&generated_did, &serialised_def, &input.private_key, &input.identity).await?;

      let serialised_result = serde_json::to_string(
        &CreateRevocationRegistryDefinitionResult {
          private_key,
          revocation_info,
          revocation_registry_definition: definition
        }
      ).unwrap();

      Ok(Some(serialised_result))
    }

    /// Issues a new credential.
    ///
    /// # Arguments
    /// * `data` - Expects a JSON object representing a `CreateRevocationRegistryDefinitionArguments`
    ///
    /// # Returns
    /// * `Option<String>` - A JSON object consisting of the credential, this credential's initial revocation state and
    /// the updated revocation info, only interesting for the issuer (needs to be stored privately)
    async fn issue_credential(&self, data: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let input: IssueCredentialArguments = serde_json::from_str(&data)?;

        // Resolve credential definition
        let definition: CredentialDefinition = serde_json::from_str(
           &self.vade.get_did_document(
             &input.credential_request.credential_definition
           ).await?
        ).unwrap();

        // Resolve schema
        let schema: CredentialSchema = serde_json::from_str(
           &self.vade.get_did_document(
             &definition.schema
           ).await?
        ).unwrap();

        // Resolve revocation definition
        let mut revocation_definition: RevocationRegistryDefinition = serde_json::from_str(
           &self.vade.get_did_document(
             &input.credential_revocation_definition
           ).await?
        ).unwrap();

        let (credential, revocation_state, revocation_info) = Issuer::issue_credential(
            &input.issuer,
            &input.subject,
            input.credential_request,
            definition,
            input.credential_private_key,
            schema,
            &mut revocation_definition,
            input.revocation_private_key,
            &input.revocation_information
        ).unwrap();



        Ok(
          Some(
            serde_json::to_string(
              &IssueCredentialResult {
                credential,
                revocation_state,
                revocation_info
              }
            ).unwrap()
          )
        )
    }

    /// Creates a `CredentialOffer` message.
    ///
    /// # Arguments
    /// * `data` - Expects a JSON object representing a `OfferCredentialArguments` type
    ///
    /// # Returns
    /// * `Option<String>` - The offer as a JSON object
    async fn create_credential_offer(&self, data: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let input: OfferCredentialArguments = serde_json::from_str(&data)?;
        let result: CredentialOffer = Issuer::offer_credential(
            &input.issuer,
            &input.subject,
            &input.schema,
            &input.credential_definition,
        );
        Ok(Some(serde_json::to_string(&result).unwrap()))
    }

    /// Creates a `CredentialOffer` message.
    ///
    /// # Arguments
    /// * `data` - Expects a JSON object representing a `PresentProofArguments` type
    ///
    /// # Returns
    /// * `Option<String>` - The offer as a JSON object
    async fn present_proof(&self, data: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let input: PresentProofArguments = serde_json::from_str(&data)?;

        // Resolve all necessary credential definitions, schemas and registries
        let mut definitions: HashMap<String, CredentialDefinition> = HashMap::new();
        let mut schemas: HashMap<String, CredentialSchema> = HashMap::new();
        let mut revocation_definitions: HashMap<String, RevocationRegistryDefinition> = HashMap::new();
        for req in &input.proof_request.sub_proof_requests {
          // Resolve schema
          let schema_did = &req.schema;
          schemas.insert(schema_did.clone(), serde_json::from_str(
             &self.vade.get_did_document(
               &schema_did
             ).await?
          ).unwrap());

          // Resolve credential definition
          let definition_did = input.credentials.get(schema_did).unwrap().signature.credential_definition.clone();
          definitions.insert(schema_did.clone(), serde_json::from_str(
             &self.vade.get_did_document(
               &definition_did
           ).await?
          ).unwrap());

          // Resolve revocation definition
          let rev_definition_did = input.credentials.get(schema_did).unwrap().signature.revocation_registry_definition.clone();
          revocation_definitions.insert(schema_did.clone(), serde_json::from_str(
             &self.vade.get_did_document(
               &rev_definition_did
             ).await?
          ).unwrap());
        }

        let result: ProofPresentation = Prover::present_proof(
            input.proof_request,
            input.credentials,
            definitions,
            schemas,
            revocation_definitions,
            input.witnesses,
            &input.master_secret,
        );

        Ok(Some(serde_json::to_string(&result).unwrap()))
    }

    /// Creates a `CredentialProposal` message.
    ///
    /// # Arguments
    /// * `data` - Expects a JSON object representing a `CreateCredentialProposalArguments` type
    ///
    /// # Returns
    /// * `Option<String>` - The proposal as a JSON object
    async fn create_credential_proposal(&self, data: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let input: CreateCredentialProposalArguments = serde_json::from_str(&data)?;
        let result: CredentialProposal = Prover::propose_credential(
            &input.issuer,
            &input.subject,
            &input.schema,
        );

        Ok(Some(serde_json::to_string(&result).unwrap()))
    }

    /// Creates a `CredentialRequest` message.
    ///
    /// # Arguments
    /// * `data` - Expects a JSON object representing a `RequestCredentialArguments` type
    ///
    /// # Returns
    /// * `Option<String>` - A JSON object consisting of the `CredentialRequest` and `CredentialSecretsBlindingFactors` (to be stored at the prover's site in a private manner)
    async fn request_credential(&self, data: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let input: RequestCredentialArguments = serde_json::from_str(&data)?;

        // Resolve credential definition
        let definition: CredentialDefinition = serde_json::from_str(
          &self.vade.get_did_document(
            &input.credential_offering.credential_definition
          ).await?
        ).unwrap();

        let result: (CredentialRequest, CredentialSecretsBlindingFactors) = Prover::request_credential(
            input.credential_offering,
            definition,
            input.master_secret,
            input.credential_values,
        );

        Ok(Some(serde_json::to_string(&result).unwrap()))
    }

    /// Creates a `ProofRequest` message.
    ///
    /// # Arguments
    /// * `data` - Expects a JSON object representing a `RequestProofArguments` type
    ///
    /// # Returns
    /// * `Option<String>` - A `ProofRequest` as JSON
    async fn request_proof(&self, data: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let input: RequestProofArguments = serde_json::from_str(&data)?;
        let result: ProofRequest = Verifier::request_proof(
            &input.verifier_did,
            &input.prover_did,
            input.sub_proof_requests,
        );

        Ok(Some(serde_json::to_string(&result).unwrap()))
    }

    /// Revokes a credential and updates the revocation registry definition.
    ///
    /// # Arguments
    /// * `data` - Expects a JSON object representing a `RevokeCredentialArguments` type
    ///
    /// # Returns
    /// * `Option<String>` - The updated revocation registry definition as a JSON object
    async fn revoke_credential(&mut self, data: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let input: RevokeCredentialArguments = serde_json::from_str(&data)?;

        // Resolve revocation definition
        let rev_def: RevocationRegistryDefinition = serde_json::from_str(
          &self.vade.get_did_document(
            &input.revocation_registry_definition
          ).await?
        ).unwrap();

        let updated_registry = Issuer::revoke_credential(
          &input.issuer,
          &rev_def,
          input.credential_revocation_id,
          &input.issuer_public_key_did,
          &input.issuer_proving_key
        );

        let serialized = serde_json::to_string(&updated_registry).unwrap();

        self.set_did_document(&rev_def.id, &serialized, &input.private_key, &input.identity).await?;

        Ok(Some(serialized))
    }

    /// Verifies a given `ProofPresentation` in accordance to the specified `ProofRequest`
    ///
    /// # Arguments
    /// * `data` - Expects a JSON object representing a `ValidateProofArguments` type
    ///
    /// # Returns
    /// * `Option<String>` - A JSON object representing a `ProofVerification` type, specifying whether verificatin was successful
    async fn verify_proof(&self, data: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let input: ValidateProofArguments = serde_json::from_str(&data)?;

        // Resolve all necessary credential definitions, schemas and registries
        let mut definitions: HashMap<String, CredentialDefinition> = HashMap::new();
        let mut rev_definitions: HashMap<String, Option<RevocationRegistryDefinition>> = HashMap::new();
        let mut schemas: HashMap<String, CredentialSchema> = HashMap::new();
        for req in &input.proof_request.sub_proof_requests {
          // Resolve schema
          let schema_did = &req.schema;
          schemas.insert(schema_did.clone(), serde_json::from_str(
            &self.vade.get_did_document(
              &schema_did
            ).await?
          ).unwrap());
        }

        for credential in &input.presented_proof.verifiable_credential {
          // Resolve credential definition
          let definition_did = &credential.proof.credential_definition.clone();
          definitions.insert(credential.credential_schema.id.clone(), serde_json::from_str(
            &self.vade.get_did_document(
              &definition_did
            ).await?
          ).unwrap());

          let rev_definition_did = &credential.proof.revocation_registry_definition.clone();
          rev_definitions.insert(credential.credential_schema.id.clone(), Some(serde_json::from_str(
            &self.vade.get_did_document(
              &rev_definition_did
            ).await?
          ).unwrap()));
        }

        let result: ProofVerification = Verifier::verify_proof(
            input.presented_proof,
            input.proof_request,
            definitions,
            schemas,
            rev_definitions
        );

        Ok(Some(serde_json::to_string(&result).unwrap()))
    }

    async fn generate_did(&mut self, private_key: &str, identity: &str) -> Result<String, Box<dyn std::error::Error>> {
      let generate_did_message = format!(r###"{{
        "type": "generateDid",
        "data": {{
            "privateKey": "{}",
            "identity": "{}"
        }}
      }}"###, private_key, identity);
      let result = self.vade.send_message(&generate_did_message).await?;
      if result.len() == 0 {
        return Err(Box::new(SimpleError::new(format!("Could not generate DID as no listeners were registered for this method"))));
      }

      let generated_did = result[0].as_ref().unwrap().to_owned();

      Ok(generated_did)
    }


  async fn whitelist_identity(&mut self, data: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let input: WhitelistIdentityArguments = serde_json::from_str(&data)?;
    let message_str = format!(r###"{{
      "type": "whitelistIdentity",
      "data": {{
        "privateKey": "{}",
        "identity": "{}"
      }}
    }}"###, input.private_key, input.identity);

    let result = self.vade.send_message(&message_str).await?;

    if result.len() == 0 {
      return Err(Box::new(SimpleError::new(format!("Could not generate DID as no listeners were registered for this method"))));
    }

    Ok(Some("".to_string()))
  }

  async fn set_did_document(&mut self, did: &str, payload: &str, private_key: &str, identity: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let message_str = format!(r###"{{
      "type": "setDidDocument",
      "data": {{
        "did": "{}",
        "payload": "{}",
        "privateKey": "{}",
        "identity": "{}"
      }}
    }}"###, did, payload.replace("\"", "\\\""), private_key, identity);
    error!("{}", message_str);
    let result = self.vade.send_message(&message_str).await?;

    if result.len() == 0 {
      return Err(Box::new(SimpleError::new(format!("Could not set did document as no listeners were registered for this method"))));
    }

    Ok(Some("".to_string()))
  }
}
