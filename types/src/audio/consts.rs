use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;
#[derive(Debug, Clone, PartialEq)]
pub enum Voice {
    Alloy,
    Echo,
    Fable,
    Onyx,
    Nova,
    Shimmer,
    Custom(String),
}

impl Serialize for Voice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Voice::Alloy => serializer.serialize_str("alloy"),
            Voice::Echo => serializer.serialize_str("echo"),
            Voice::Fable => serializer.serialize_str("fable"),
            Voice::Onyx => serializer.serialize_str("onyx"),
            Voice::Nova => serializer.serialize_str("nova"),
            Voice::Shimmer => serializer.serialize_str("shimmer"),
            Voice::Custom(s) => serializer.serialize_str(s),
        }
    }
}


impl FromStr for Voice {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "alloy" => Voice::Alloy,
            "echo" => Voice::Echo,
            "fable" => Voice::Fable,
            "onyx" => Voice::Onyx,
            "nova" => Voice::Nova,
            "shimmer" => Voice::Shimmer,
            _ => Voice::Custom(s.to_string()),
        })
    }
}

impl<'de> Deserialize<'de> for Voice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Voice::from_str(&s).unwrap())
    }
}


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum AudioFormat {
    #[serde(rename = "pcm16")]
    Pcm16,
    #[serde(rename = "g711_ulaw")]
    Mulaw,
    #[serde(rename = "g711_alaw")]
    Alaw,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TranscriptionModel {
    Whisper,
    Custom(String),
}

impl Serialize for TranscriptionModel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            TranscriptionModel::Whisper => serializer.serialize_str("whisper-1"),
            TranscriptionModel::Custom(s) => serializer.serialize_str(s),
        }
    }
}

impl FromStr for TranscriptionModel {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "whisper-1" => TranscriptionModel::Whisper,
            _ => TranscriptionModel::Custom(s.to_string()),
        })
    }
}

impl<'de> Deserialize<'de> for TranscriptionModel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(TranscriptionModel::from_str(&s).unwrap())
    }
}

mod test {

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct AudioConsts {
        #[serde(skip_serializing_if = "Option::is_none")]
        voice: Option<super::Voice>,
        #[serde(skip_serializing_if = "Option::is_none")]
        audio_format: Option<super::AudioFormat>,
    }

    #[test]
    fn test_serialize() {
        let consts = AudioConsts {
            voice: Some(super::Voice::Alloy),
            audio_format: Some(super::AudioFormat::Pcm16),
        };
        let json = serde_json::to_string(&consts).unwrap();
        let expected = r#"{"voice":"alloy","audio_format":"pcm16"}"#;
        assert_eq!(json, expected);

        let consts = AudioConsts {
            voice: Some(super::Voice::Custom("taro".to_string())),
            audio_format: None,
        };
        let json = serde_json::to_string(&consts).unwrap();
        let expected = r#"{"voice":"taro"}"#;
        assert_eq!(json, expected);

        let consts = AudioConsts {
            voice: None,
            audio_format: Some(super::AudioFormat::Mulaw),
        };
        let json = serde_json::to_string(&consts).unwrap();
        let expected = r#"{"audio_format":"g711_ulaw"}"#;
        assert_eq!(json, expected);
    }

    #[test]
    fn test_deserialize() {
        let json = r#"{"voice":"alloy","audio_format":"pcm16"}"#;
        let consts: AudioConsts = serde_json::from_str(json).unwrap();
        assert_eq!(consts.voice, Some(super::Voice::Alloy));
        assert_eq!(consts.audio_format, Some(super::AudioFormat::Pcm16));

        let json = r#"{"voice":"emi"}"#;
        let consts: AudioConsts = serde_json::from_str(json).unwrap();
        assert_eq!(consts.voice, Some(super::Voice::Custom("emi".to_string())));
        assert_eq!(consts.audio_format, None);

        let json = r#"{"audio_format":"g711_ulaw"}"#;
        let consts: AudioConsts = serde_json::from_str(json).unwrap();
        assert_eq!(consts.voice, None);
        assert_eq!(consts.audio_format, Some(super::AudioFormat::Mulaw));
    }
}