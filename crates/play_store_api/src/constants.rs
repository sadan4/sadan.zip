pub mod clusters {
	pub const NEW: &str = "new";
	pub const TOP: &str = "top";
}

pub mod category {
	pub const APPLICATION: &str = "APPLICATION";
	pub const ANDROID_WEAR: &str = "ANDROID_WEAR";
	pub const ART_AND_DESIGN: &str = "ART_AND_DESIGN";
	pub const AUTO_AND_VEHICLES: &str = "AUTO_AND_VEHICLES";
	pub const BEAUTY: &str = "BEAUTY";
	pub const BOOKS_AND_REFERENCE: &str = "BOOKS_AND_REFERENCE";
	pub const BUSINESS: &str = "BUSINESS";
	pub const COMICS: &str = "COMICS";
	pub const COMMUNICATION: &str = "COMMUNICATION";
	pub const DATING: &str = "DATING";
	pub const EDUCATION: &str = "EDUCATION";
	pub const ENTERTAINMENT: &str = "ENTERTAINMENT";
	pub const EVENTS: &str = "EVENTS";
	pub const FINANCE: &str = "FINANCE";
	pub const FOOD_AND_DRINK: &str = "FOOD_AND_DRINK";
	pub const HEALTH_AND_FITNESS: &str = "HEALTH_AND_FITNESS";
	pub const HOUSE_AND_HOME: &str = "HOUSE_AND_HOME";
	pub const LIBRARIES_AND_DEMO: &str = "LIBRARIES_AND_DEMO";
	pub const LIFESTYLE: &str = "LIFESTYLE";
	pub const MAPS_AND_NAVIGATION: &str = "MAPS_AND_NAVIGATION";
	pub const MEDICAL: &str = "MEDICAL";
	pub const MUSIC_AND_AUDIO: &str = "MUSIC_AND_AUDIO";
	pub const NEWS_AND_MAGAZINES: &str = "NEWS_AND_MAGAZINES";
	pub const PARENTING: &str = "PARENTING";
	pub const PERSONALIZATION: &str = "PERSONALIZATION";
	pub const PHOTOGRAPHY: &str = "PHOTOGRAPHY";
	pub const PRODUCTIVITY: &str = "PRODUCTIVITY";
	pub const SHOPPING: &str = "SHOPPING";
	pub const SOCIAL: &str = "SOCIAL";
	pub const SPORTS: &str = "SPORTS";
	pub const TOOLS: &str = "TOOLS";
	pub const TRAVEL_AND_LOCAL: &str = "TRAVEL_AND_LOCAL";
	pub const VIDEO_PLAYERS: &str = "VIDEO_PLAYERS";
	pub const WATCH_FACE: &str = "WATCH_FACE";
	pub const WEATHER: &str = "WEATHER";
	pub const GAME: &str = "GAME";
	pub const GAME_ACTION: &str = "GAME_ACTION";
	pub const GAME_ADVENTURE: &str = "GAME_ADVENTURE";
	pub const GAME_ARCADE: &str = "GAME_ARCADE";
	pub const GAME_BOARD: &str = "GAME_BOARD";
	pub const GAME_CARD: &str = "GAME_CARD";
	pub const GAME_CASINO: &str = "GAME_CASINO";
	pub const GAME_CASUAL: &str = "GAME_CASUAL";
	pub const GAME_EDUCATIONAL: &str = "GAME_EDUCATIONAL";
	pub const GAME_MUSIC: &str = "GAME_MUSIC";
	pub const GAME_PUZZLE: &str = "GAME_PUZZLE";
	pub const GAME_RACING: &str = "GAME_RACING";
	pub const GAME_ROLE_PLAYING: &str = "GAME_ROLE_PLAYING";
	pub const GAME_SIMULATION: &str = "GAME_SIMULATION";
	pub const GAME_SPORTS: &str = "GAME_SPORTS";
	pub const GAME_STRATEGY: &str = "GAME_STRATEGY";
	pub const GAME_TRIVIA: &str = "GAME_TRIVIA";
	pub const GAME_WORD: &str = "GAME_WORD";
	pub const FAMILY: &str = "FAMILY";
}

pub mod collection {
	pub const TOP_FREE: &str = "TOP_FREE";
	pub const TOP_PAID: &str = "TOP_PAID";
	pub const GROSSING: &str = "GROSSING";
}

pub mod sort {
	pub const NEWEST: u8 = 2;
	pub const RATING: u8 = 3;
	pub const HELPFULNESS: u8 = 1;
}

pub mod age {
	pub const FIVE_UNDER: &str = "AGE_RANGE1";
	pub const SIX_EIGHT: &str = "AGE_RANGE2";
	pub const NINE_UP: &str = "AGE_RANGE3";
}

pub mod permission {
	pub const COMMON: u8 = 0;
	pub const OTHER: u8 = 1;
}

pub const BASE_URL: &str = "https://play.google.com";

pub const MAX_REDIRECTS: usize = 10;
