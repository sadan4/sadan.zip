use super::*;
mod wreq_d {
	use super::*;
	use macros::test;
	#[test]
	fn simple_modules() {
		let p = parse!("test_data/wp/module.js");
		let export_map = p.dbg_export_map();
		assert_debug_snapshot!(export_map, @r#"
		{
		    "TB": [
		        "[4:8->4:10) TB",
		        "[162:13->162:14) T",
		    ],
		    "VY": [
		        "[5:8->5:10) VY",
		        "[183:13->183:14) x",
		    ],
		    "ZP": [
		        "[6:8->6:10) ZP",
		        "[87:13->87:14) y",
		    ],
		}
		"#);
	}
	#[test]
	fn string_literal_export() {
		let p = parse!("test_data/wp/wreq.d/simpleString.js");
		let export_map = p.dbg_export_map();
		assert_debug_snapshot!(export_map, @r#"
		{
		    "STRING_EXPORT": "47835198259242069"(
		        [
		            "[5:8->5:21) STRING_EXPORT",
		            "[7:12->7:31) \\\"47835198259242069\\\"",
		        ],
		    ),
		}
		"#);
	}

	#[test]
	fn object_literal_export() {
		let p = parse!("test_data/wp/wreq.d/objectExport.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "EO": [
		        "[5:8->5:10) EO",
		        "[124:13->124:14) T",
		    ],
		    "ZP": ExportMap(
		        {
		            "getFormattedName": [
		                "[164:8->164:24) getFormattedName",
		                "[81:13->81:14) v",
		            ],
		            "getGlobalName": [
		                "[165:8->165:21) getGlobalName",
		                "[72:13->72:14) y",
		            ],
		            "getName": [
		                "[156:8->156:15) getName",
		                "[53:13->53:14) E",
		            ],
		            "getUserTag": [
		                "[159:8->159:18) getUserTag",
		                "[142:13->142:14) A",
		            ],
		            "humanizeStatus": [
		                "[166:8->166:22) humanizeStatus",
		                "[90:13->90:14) O",
		            ],
		            "isNameConcealed": [
		                "[158:8->158:23) isNameConcealed",
		                "[158:25->158:30) e => ",
		            ],
		            "useDirectMessageRecipient": [
		                "[167:8->167:33) useDirectMessageRecipient",
		                "[147:13->147:14) C",
		            ],
		            "useName": [
		                "[157:8->157:15) useName",
		                "[62:13->62:14) b",
		            ],
		            "useUserTag": [
		                "[160:8->160:18) useUserTag",
		                "[160:20->160:35) function(e, t) ",
		            ],
		        },
		    ),
		}
		"#);
	}

	#[test]
	fn object_with_computed_prop() {
		let p = parse!("test_data/wp/wreq.d/computedPropInObj.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "Z": ExportMap(
		        {
		            "n(231338).Et.GET_PLATFORM_BEHAVIORS": ExportMap(
		                {
		                    "handler": [
		                        "[8:12->8:19) handler",
		                        "[8:21->8:27) () => ",
		                    ],
		                },
		            ),
		        },
		    ),
		}
		"#);
	}

	#[test]
	fn class_export() {
		let p = parse!("test_data/wp/wreq.d/classExport.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "U": ExportMap(
		        {
		            "_dispatch": [
		                "[112:8->112:17) _dispatch",
		            ],
		            "_dispatchWithDevtools": [
		                "[88:8->88:29) _dispatchWithDevtools",
		            ],
		            "_dispatchWithLogging": [
		                "[91:8->91:28) _dispatchWithLogging",
		            ],
		            "addDependencies": [
		                "[150:8->150:23) addDependencies",
		            ],
		            "addInterceptor": [
		                "[127:8->127:22) addInterceptor",
		            ],
		            "createToken": [
		                "[147:8->147:19) createToken",
		            ],
		            "dispatch": [
		                "[38:8->38:16) dispatch",
		            ],
		            "dispatchForStoreTest": [
		                "[55:8->55:28) dispatchForStoreTest",
		            ],
		            "flushWaitQueue": [
		                "[60:8->60:22) flushWaitQueue",
		            ],
		            "isDispatching": [
		                "[35:8->35:21) isDispatching",
		            ],
		            "register": [
		                "[144:8->144:16) register",
		            ],
		            "subscribe": [
		                "[134:8->134:17) subscribe",
		            ],
		            "unsubscribe": [
		                "[139:8->139:19) unsubscribe",
		            ],
		            "wait": [
		                "[130:8->130:12) wait",
		            ],
		            "SYM_CJS_DEFAULT": [
		                "[5:8->5:9) U",
		                "[34:10->34:11) E",
		                "[153:8->153:19) constructor",
		            ],
		        },
		    ),
		}
		"#);
	}

	#[test]
	fn enum_export() {
		let p = parse!("test_data/wp/wreq.d/enums.js");
		let map = p.get_export_map();
		// only pick the keys we have tests for in js
		// TODO: Broaden tests in this module
		let mut map2 = map.clone();
		map2.exports.retain(|k, _| {
			matches!(k.as_str(), "$7" | "$X" | "$n" | "C" | "Cj" | "Si")
		});
		let map2_dumper = ExportMapDumper(&map2, p.source);
		assert_debug_snapshot!(map2_dumper, @r#"
		{
		    "$7": 28(
		        [
		            "[5:8->5:10) $7",
		            "[385:12->385:14) 28",
		        ],
		    ),
		    "$X": "1397626558063050855"(
		        [
		            "[7:8->7:10) $X",
		            "[421:13->421:34) \\\"1397626558063050855\\\"",
		        ],
		    ),
		    "$n": 190(
		        [
		            "[9:8->9:10) $n",
		            "[739:13->739:16) 190",
		        ],
		    ),
		    "C": ExportMap(
		        {
		            "PREMIUM_DISCOUNT": 1(
		                [
		                    "[118:12->118:28) PREMIUM_DISCOUNT",
		                    "[118:31->118:32) 1",
		                ],
		            ),
		            "PREMIUM_TRIAL": 0(
		                [
		                    "[117:19->117:32) PREMIUM_TRIAL",
		                    "[117:35->117:36) 0",
		                ],
		            ),
		            "SYM_CJS_DEFAULT": [
		                "[13:8->13:9) C",
		                "[116:8->116:9) s",
		            ],
		        },
		    ),
		    "Cj": ExportMap(
		        {
		            "BOX": 2(
		                [
		                    "[701:12->701:15) BOX",
		                    "[701:18->701:19) 2",
		                ],
		            ),
		            "CAKE": 5(
		                [
		                    "[704:12->704:16) CAKE",
		                    "[704:19->704:20) 5",
		                ],
		            ),
		            "CHEST": 6(
		                [
		                    "[705:12->705:17) CHEST",
		                    "[705:20->705:21) 6",
		                ],
		            ),
		            "COFFEE": 7(
		                [
		                    "[706:12->706:18) COFFEE",
		                    "[706:21->706:22) 7",
		                ],
		            ),
		            "CUP": 3(
		                [
		                    "[702:12->702:15) CUP",
		                    "[702:18->702:19) 3",
		                ],
		            ),
		            "NITROWEEN_STANDARD": 12(
		                [
		                    "[711:12->711:30) NITROWEEN_STANDARD",
		                    "[711:33->711:35) 12",
		                ],
		            ),
		            "SEASONAL_CAKE": 9(
		                [
		                    "[708:12->708:25) SEASONAL_CAKE",
		                    "[708:28->708:29) 9",
		                ],
		            ),
		            "SEASONAL_CHEST": 10(
		                [
		                    "[709:12->709:26) SEASONAL_CHEST",
		                    "[709:29->709:31) 10",
		                ],
		            ),
		            "SEASONAL_COFFEE": 11(
		                [
		                    "[710:12->710:27) SEASONAL_COFFEE",
		                    "[710:30->710:32) 11",
		                ],
		            ),
		            "SEASONAL_STANDARD_BOX": 8(
		                [
		                    "[707:12->707:33) SEASONAL_STANDARD_BOX",
		                    "[707:36->707:37) 8",
		                ],
		            ),
		            "SNOWGLOBE": 1(
		                [
		                    "[700:19->700:28) SNOWGLOBE",
		                    "[700:31->700:32) 1",
		                ],
		            ),
		            "STANDARD_BOX": 4(
		                [
		                    "[703:12->703:24) STANDARD_BOX",
		                    "[703:27->703:28) 4",
		                ],
		            ),
		            "SYM_CJS_DEFAULT": [
		                "[17:8->17:10) Cj",
		                "[699:8->699:10) eY",
		            ],
		        },
		    ),
		    "Si": ExportMap(
		        {
		            "GUILD": "590663762298667008"(
		                [
		                    "[153:10->153:15) GUILD",
		                    "[153:18->153:38) \\\"590663762298667008\\\"",
		                ],
		            ),
		            "LEGACY": "521842865731534868"(
		                [
		                    "[154:10->154:16) LEGACY",
		                    "[154:19->154:39) \\\"521842865731534868\\\"",
		                ],
		            ),
		            "NONE": "628379670982688768"(
		                [
		                    "[149:17->149:21) NONE",
		                    "[149:24->149:44) \\\"628379670982688768\\\"",
		                ],
		            ),
		            "TIER_0": "978380684370378762"(
		                [
		                    "[150:10->150:16) TIER_0",
		                    "[150:19->150:39) \\\"978380684370378762\\\"",
		                ],
		            ),
		            "TIER_1": "521846918637420545"(
		                [
		                    "[151:10->151:16) TIER_1",
		                    "[151:19->151:39) \\\"521846918637420545\\\"",
		                ],
		            ),
		            "TIER_2": "521847234246082599"(
		                [
		                    "[152:10->152:16) TIER_2",
		                    "[152:19->152:39) \\\"521847234246082599\\\"",
		                ],
		            ),
		            "SYM_CJS_DEFAULT": [
		                "[44:8->44:10) Si",
		                "[148:8->148:9) _",
		            ],
		        },
		    ),
		}
		"#);
	}

	#[test]
	fn namespace_enum_export() {
		let p = parse!("test_data/wp/wreq.d/enums2.js");
		let map = p.get_export_map();
		// filter the single `Z` export's inner map to a handful of members
		let mut map2 = map.clone();
		if let Some(ExportValue::Map(inner)) = map2.exports.get_mut("Z") {
			inner.exports.retain(|k, _| {
				matches!(
					k.as_str(),
					"QUICK_SWITCHER"
						| "POPOUT_WINDOW"
						| "OVERLAY"
						| "NOTICE"
						| "BADGE"
						| "CF_WARP_SETTINGS"
				)
			});
		}
		let map2_dumper = ExportMapDumper(&map2, p.source);
		assert_debug_snapshot!(map2_dumper, @r#"
		{
		    "Z": ExportMap(
		        {
		            "BADGE": "badge"(
		                [
		                    "[14:10->14:15) BADGE",
		                    "[14:18->14:25) \\\"badge\\\"",
		                ],
		            ),
		            "CF_WARP_SETTINGS": "cloudflare warp settings"(
		                [
		                    "[537:10->537:26) CF_WARP_SETTINGS",
		                    "[537:29->537:55) \\\"cloudflare warp settings\\\"",
		                ],
		            ),
		            "NOTICE": "notice"(
		                [
		                    "[12:10->12:16) NOTICE",
		                    "[12:19->12:27) \\\"notice\\\"",
		                ],
		            ),
		            "OVERLAY": "overlay"(
		                [
		                    "[11:10->11:17) OVERLAY",
		                    "[11:20->11:29) \\\"overlay\\\"",
		                ],
		            ),
		            "POPOUT_WINDOW": "popout window"(
		                [
		                    "[10:10->10:23) POPOUT_WINDOW",
		                    "[10:26->10:41) \\\"popout window\\\"",
		                ],
		            ),
		            "QUICK_SWITCHER": "quick switcher"(
		                [
		                    "[9:17->9:31) QUICK_SWITCHER",
		                    "[9:34->9:50) \\\"quick switcher\\\"",
		                ],
		            ),
		            "SYM_CJS_DEFAULT": [
		                "[6:8->6:9) Z",
		                "[8:8->8:9) r",
		            ],
		        },
		    ),
		}
		"#);
	}

	#[test]
	fn seq_expr_enum_export() {
		let p = parse!("test_data/wp/wreq.d/enums3.js");
		let map = p.get_export_map();
		let map_dumper = ExportMapDumper(map, p.source);
		assert_debug_snapshot!(map_dumper, @r#"
		{
		    "n": ExportMap(
		        {
		            "EXACT": "exact"(
		                [
		                    "[8:6->8:11) EXACT",
		                    "[8:14->8:21) \\\"exact\\\"",
		                ],
		            ),
		            "FUZZY": "fuzzy"(
		                [
		                    "[7:28->7:33) FUZZY",
		                    "[7:36->7:43) \\\"fuzzy\\\"",
		                ],
		            ),
		            "JARO_WINKLER": "jaro_winkler"(
		                [
		                    "[10:6->10:18) JARO_WINKLER",
		                    "[10:21->10:35) \\\"jaro_winkler\\\"",
		                ],
		            ),
		            "REGEX": "regex"(
		                [
		                    "[9:6->9:11) REGEX",
		                    "[9:14->9:21) \\\"regex\\\"",
		                ],
		            ),
		            "SYM_CJS_DEFAULT": [
		                "[4:8->4:9) n",
		                "[7:14->7:15) a",
		            ],
		        },
		    ),
		    "r": ExportMap(
		        {
		            "JARO_WINKLER": "jaro_winkler"(
		                [
		                    "[12:6->12:18) JARO_WINKLER",
		                    "[12:21->12:35) \\\"jaro_winkler\\\"",
		                ],
		            ),
		            "NONE": "none"(
		                [
		                    "[11:22->11:26) NONE",
		                    "[11:29->11:35) \\\"none\\\"",
		                ],
		            ),
		            "SYM_CJS_DEFAULT": [
		                "[5:8->5:9) r",
		                "[11:8->11:9) i",
		            ],
		        },
		    ),
		}
		"#);
	}

	#[test]
	fn seq_expr_enum_export_style_2() {
		let p = parse!("test_data/wp/wreq.d/enums4.js");
		let map = p.get_export_map();
		let map_dumper = ExportMapDumper(map, p.source);
		assert_debug_snapshot!(map_dumper, @r#"
		{
		    "n": ExportMap(
		        {
		            "CONTEXTLESS": 512(
		                [
		                    "[15:8->15:19) CONTEXTLESS",
		                    "[15:22->15:25) 512",
		                ],
		            ),
		            "EMBEDDED": 256(
		                [
		                    "[14:8->14:16) EMBEDDED",
		                    "[14:19->14:22) 256",
		                ],
		            ),
		            "INSTANCE": 1(
		                [
		                    "[8:25->8:33) INSTANCE",
		                    "[8:36->8:37) 1",
		                ],
		            ),
		            "JOIN": 2(
		                [
		                    "[9:8->9:12) JOIN",
		                    "[9:15->9:16) 2",
		                ],
		            ),
		            "PARTY_PRIVACY_FRIENDS": 64(
		                [
		                    "[12:8->12:29) PARTY_PRIVACY_FRIENDS",
		                    "[12:32->12:34) 64",
		                ],
		            ),
		            "PARTY_PRIVACY_VOICE_CHANNEL": 128(
		                [
		                    "[13:8->13:35) PARTY_PRIVACY_VOICE_CHANNEL",
		                    "[13:38->13:41) 128",
		                ],
		            ),
		            "PLAY": 32(
		                [
		                    "[11:8->11:12) PLAY",
		                    "[11:15->11:17) 32",
		                ],
		            ),
		            "SUPPORTS_JOIN_URL": 2048(
		                [
		                    "[17:8->17:25) SUPPORTS_JOIN_URL",
		                    "[17:28->17:32) 2048",
		                ],
		            ),
		            "SUPPORTS_REMOTE_ACTIVITY_ACTION_JOIN": 1024(
		                [
		                    "[16:8->16:44) SUPPORTS_REMOTE_ACTIVITY_ACTION_JOIN",
		                    "[16:47->16:51) 1024",
		                ],
		            ),
		            "SYNC": 16(
		                [
		                    "[10:8->10:12) SYNC",
		                    "[10:15->10:17) 16",
		                ],
		            ),
		            "SYM_CJS_DEFAULT": [
		                "[4:8->4:9) n",
		                "[8:8->8:10) nF",
		            ],
		        },
		    ),
		    "r": ExportMap(
		        {
		            "ALL_MESSAGES": 0(
		                [
		                    "[19:25->19:37) ALL_MESSAGES",
		                    "[19:40->19:41) 0",
		                ],
		            ),
		            "NO_MESSAGES": 2(
		                [
		                    "[21:8->21:19) NO_MESSAGES",
		                    "[21:22->21:23) 2",
		                ],
		            ),
		            "NULL": 3(
		                [
		                    "[22:8->22:12) NULL",
		                    "[22:15->22:16) 3",
		                ],
		            ),
		            "ONLY_MENTIONS": 1(
		                [
		                    "[20:8->20:21) ONLY_MENTIONS",
		                    "[20:24->20:25) 1",
		                ],
		            ),
		            "SYM_CJS_DEFAULT": [
		                "[5:8->5:9) r",
		                "[19:8->19:10) nV",
		            ],
		        },
		    ),
		}
		"#);
	}

	#[test]
	fn seq_expr_enum_export_style_3() {
		let p = parse!("test_data/wp/wreq.d/enums5.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "A": ExportMap(
		        {
		            "NOTICE": "notice"(
		                [
		                    "[10:6->10:12) NOTICE",
		                    "[10:15->10:23) \\\"notice\\\"",
		                ],
		            ),
		            "OVERLAY": "overlay"(
		                [
		                    "[9:6->9:13) OVERLAY",
		                    "[9:16->9:25) \\\"overlay\\\"",
		                ],
		            ),
		            "POPOUT_WINDOW": "popout window"(
		                [
		                    "[8:6->8:19) POPOUT_WINDOW",
		                    "[8:22->8:37) \\\"popout window\\\"",
		                ],
		            ),
		            "PREMIUM_UPSELL_TOOLTIP": "premium upsell tooltip"(
		                [
		                    "[11:6->11:28) PREMIUM_UPSELL_TOOLTIP",
		                    "[11:31->11:55) \\\"premium upsell tooltip\\\"",
		                ],
		            ),
		            "QUICK_SWITCHER": "quick switcher"(
		                [
		                    "[7:30->7:44) QUICK_SWITCHER",
		                    "[7:47->7:63) \\\"quick switcher\\\"",
		                ],
		            ),
		            "XBOX_PERKS_CONNECTION_FOOTER": "xbox perks connection footer"(
		                [
		                    "[12:6->12:34) XBOX_PERKS_CONNECTION_FOOTER",
		                    "[12:37->12:67) \\\"xbox perks connection footer\\\"",
		                ],
		            ),
		            "XBOX_PERKS_MODAL": "xbox perks modal"(
		                [
		                    "[13:6->13:22) XBOX_PERKS_MODAL",
		                    "[13:25->13:43) \\\"xbox perks modal\\\"",
		                ],
		            ),
		            "SYM_CJS_DEFAULT": [
		                "[5:8->5:9) A",
		                "[7:11->7:12) r",
		            ],
		        },
		    ),
		}
		"#);
	}

	#[test]
	fn object_freeze_enum_export() {
		let p = parse!("test_data/wp/wreq.d/objectFreeze.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "k": ExportMap(
		        {
		            "ALL": null(
		                [
		                    "[9:8->9:11) ALL",
		                    "[9:13->9:17) null",
		                ],
		            ),
		            "CHANNEL_CREATE": 10(
		                [
		                    "[11:8->11:22) CHANNEL_CREATE",
		                    "[11:24->11:26) 10",
		                ],
		            ),
		            "CHANNEL_DELETE": 12(
		                [
		                    "[13:8->13:22) CHANNEL_DELETE",
		                    "[13:24->13:26) 12",
		                ],
		            ),
		            "CHANNEL_UPDATE": 11(
		                [
		                    "[12:8->12:22) CHANNEL_UPDATE",
		                    "[12:24->12:26) 11",
		                ],
		            ),
		            "GUILD_UPDATE": 1(
		                [
		                    "[10:8->10:20) GUILD_UPDATE",
		                    "[10:22->10:23) 1",
		                ],
		            ),
		        },
		    ),
		    "l": ExportMap(
		        {
		            "0": 0(
		                [
		                    "[16:8->16:9) 0",
		                    "[16:11->16:12) 0",
		                ],
		            ),
		            "1": 2(
		                [
		                    "[17:8->17:9) 1",
		                    "[17:11->17:12) 2",
		                ],
		            ),
		            "2": 7(
		                [
		                    "[18:8->18:9) 2",
		                    "[18:11->18:12) 7",
		                ],
		            ),
		            "3": 14(
		                [
		                    "[19:8->19:9) 3",
		                    "[19:11->19:13) 14",
		                ],
		            ),
		        },
		    ),
		}
		"#);
	}
}
mod e_exports {
	use super::*;
	use macros::test;
	#[test]
	/// class names
	fn object_literal_exports() {
		let p = parse!("test_data/wp/e.exports/objLiteral.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "addButton": "addButton_f5cb44"(
		        [
		            "[7:8->7:17) addButton",
		            "[7:19->7:37) \\\"addButton_f5cb44\\\"",
		        ],
		    ),
		    "addButtonInner": "addButtonInner_f5cb44"(
		        [
		            "[8:8->8:22) addButtonInner",
		            "[8:24->8:47) \\\"addButtonInner_f5cb44\\\"",
		        ],
		    ),
		    "productListings": "productListings_f5cb44"(
		        [
		            "[6:8->6:23) productListings",
		            "[6:25->6:49) \\\"productListings_f5cb44\\\"",
		        ],
		    ),
		    "productListingsHeader": "productListingsHeader_f5cb44"(
		        [
		            "[5:8->5:29) productListingsHeader",
		            "[5:31->5:61) \\\"productListingsHeader_f5cb44\\\"",
		        ],
		    ),
		}
		"#);
	}
	#[test]
	fn single_string_export() {
		let p = parse!("test_data/wp/e.exports/string.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "SYM_CJS_DEFAULT": "/assets/b8deed70d3e4a9bd.svg"(
		        [
		            "[4:16->4:46) \\\"/assets/b8deed70d3e4a9bd.svg\\\"",
		        ],
		    ),
		}
		"#);
	}
	#[test]
	fn re_export() {
		let p = parse!("test_data/wp/e.exports/identReExport.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "SYM_CJS_DEFAULT": [
		        "[4:12->4:21) n(843767)",
		    ],
		}
		"#);
	}
	#[test]
	fn exports_with_an_intermediate_var() {
		let p = parse!("test_data/wp/e.exports/ident.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "closeContainer": "closeContainer__2dea3"(
		        [
		            "[6:8->6:22) closeContainer",
		            "[6:24->6:47) \\\"closeContainer__2dea3\\\"",
		        ],
		    ),
		    "closeIcon": "closeIcon__2dea3"(
		        [
		            "[7:8->7:17) closeIcon",
		            "[7:19->7:37) \\\"closeIcon__2dea3\\\"",
		        ],
		    ),
		    "confirmationContainer": "confirmationContainer__2dea3"(
		        [
		            "[10:8->10:29) confirmationContainer",
		            "[10:31->10:61) \\\"confirmationContainer__2dea3\\\"",
		        ],
		    ),
		    "confirmationSubtitle": "confirmationSubtitle__2dea3"(
		        [
		            "[13:8->13:28) confirmationSubtitle",
		            "[13:30->13:59) \\\"confirmationSubtitle__2dea3\\\"",
		        ],
		    ),
		    "confirmationTitle": "confirmationTitle__2dea3"(
		        [
		            "[12:8->12:25) confirmationTitle",
		            "[12:27->12:53) \\\"confirmationTitle__2dea3\\\"",
		        ],
		    ),
		    "headerContainer": "headerContainer__2dea3"(
		        [
		            "[5:8->5:23) headerContainer",
		            "[5:25->5:49) \\\"headerContainer__2dea3\\\"",
		        ],
		    ),
		    "headerImage": "headerImage__2dea3"(
		        [
		            "[8:8->8:19) headerImage",
		            "[8:21->8:41) \\\"headerImage__2dea3\\\"",
		        ],
		    ),
		    "headerImageContainer": "headerImageContainer__2dea3"(
		        [
		            "[9:8->9:28) headerImageContainer",
		            "[9:30->9:59) \\\"headerImageContainer__2dea3\\\"",
		        ],
		    ),
		    "purchaseConfirmation": "purchaseConfirmation__2dea3 confirmationContainer__2dea3"(
		        [
		            "[11:8->11:28) purchaseConfirmation",
		            "[11:30->11:88) \\\"purchaseConfirmation__2dea3 confirmationContainer__2dea3\\\"",
		        ],
		    ),
		}
		"#);
	}
	#[test]
	fn function_expression() {
		let p = parse!("test_data/wp/e.exports/function.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "SYM_CJS_DEFAULT": [
		        "[9:16->9:28) function(e) ",
		    ],
		}
		"#);
	}
	#[test]
	fn class_default_export() {
		let p = parse!("test_data/wp/e.exports/classExport.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "_dispatch": [
		        "[112:8->112:17) _dispatch",
		    ],
		    "_dispatchWithDevtools": [
		        "[88:8->88:29) _dispatchWithDevtools",
		    ],
		    "_dispatchWithLogging": [
		        "[91:8->91:28) _dispatchWithLogging",
		    ],
		    "addDependencies": [
		        "[150:8->150:23) addDependencies",
		    ],
		    "addInterceptor": [
		        "[127:8->127:22) addInterceptor",
		    ],
		    "createToken": [
		        "[147:8->147:19) createToken",
		    ],
		    "dispatch": [
		        "[38:8->38:16) dispatch",
		    ],
		    "dispatchForStoreTest": [
		        "[55:8->55:28) dispatchForStoreTest",
		    ],
		    "flushWaitQueue": [
		        "[60:8->60:22) flushWaitQueue",
		    ],
		    "isDispatching": [
		        "[35:8->35:21) isDispatching",
		    ],
		    "register": [
		        "[144:8->144:16) register",
		    ],
		    "subscribe": [
		        "[134:8->134:17) subscribe",
		    ],
		    "unsubscribe": [
		        "[139:8->139:19) unsubscribe",
		    ],
		    "wait": [
		        "[130:8->130:12) wait",
		    ],
		    "SYM_CJS_DEFAULT": [
		        "[34:10->34:11) E",
		        "[153:8->153:19) constructor",
		    ],
		}
		"#);
	}
	#[test]
	/// `parses_everything_else` from js
	fn ponyfill() {
		let p = parse!("test_data/wp/e.exports/everythingElse.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "SYM_CJS_DEFAULT": [
		        "[5:16->5:44) Function.prototype.bind || r",
		    ],
		}
		"#);
	}

	#[test]
	fn e_exports_on_rhs() {
		let p = parse!("test_data/wp/e.exports/panic1.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "__esModule": [
		        "[9:27->9:29) !0",
		    ],
		    "default": [
		        "[10:24->10:33) e.exports",
		    ],
		    "SYM_CJS_DEFAULT": [
		        "[4:16->4:28) function(e) ",
		    ],
		}
		"#);
	}

	#[test]
	fn runtime_export_switch() {
		let p = parse!("test_data/wp/e.exports/runtimeExportSwitch.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "__esModule": [
		        "[12:31->12:33) !0",
		    ],
		    "default": [
		        "[13:28->13:37) t.exports",
		    ],
		    "SYM_CJS_DEFAULT": [
		        "[7:27->11:9) s = \\\"function\\\" == typeof n && \\\"symbol\\\" == typeof o ? function(t) {\\n            return typeof t\\n        } : function(t) {\\n            return t && \\\"function\\\" == typeof n && t.constructor === n && t !== n.prototype ? \\\"symbol\\\" : typeof t\\n        }",
		    ],
		}
		"#);
	}

	#[test]
	fn intl_chunk() {
		let p = parse!("test_data/wp/e.exports/i18nModule.js");
		assert!(p.is_intl_module());
	}
}
mod exports {
	use super::*;
	use macros::test;
	#[test]
	fn pre_es6_class() {
		let p = parse!("test_data/wp/exports/module.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "Deflate": [
		        "[101:6->101:13) Deflate",
		        "[18:13->18:14) m",
		    ],
		    "deflate": [
		        "[102:6->102:13) deflate",
		        "[49:13->49:14) E",
		    ],
		    "deflateRaw": [
		        "[103:6->103:16) deflateRaw",
		        "[56:13->56:14) v",
		    ],
		    "gzip": [
		        "[104:6->104:10) gzip",
		        "[60:13->60:14) b",
		    ],
		}
		"#);
	}
}
mod stores {
	use super::*;
	use macros::test;
	#[test]
	fn normal_store() {
		let p = parse!("test_data/wp/stores/store1.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "Z": EnablePublicGuildUpsellNoticeStore(
		        {
		            "initialize": [
		                "[11:8->11:18) initialize",
		            ],
		            "isVisible": [
		                "[18:8->18:17) isVisible",
		            ],
		            "SYM_CJS_DEFAULT": [
		                "[4:8->4:9) Z",
		                "[32:16->32:17) m",
		                "[10:10->10:11) m",
		            ],
		        },
		    ),
		}
		"#);
	}
	#[test]
	fn ctor_with_no_args() {
		let p = parse!("test_data/wp/stores/store2.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "ASSISTANT_WUMPUS_VOICE_USER": "47835198259242069"(
		        [
		            "[6:8->6:35) ASSISTANT_WUMPUS_VOICE_USER",
		            "[39:12->39:31) \\\"47835198259242069\\\"",
		        ],
		    ),
		    "default": UserStore(
		        {
		            "filter": [
		                "[260:8->260:14) filter",
		            ],
		            "findByTag": [
		                "[253:8->253:17) findByTag",
		            ],
		            "forEach": [
		                "[248:8->248:15) forEach",
		            ],
		            "getCurrentUser": [
		                "[270:8->270:22) getCurrentUser",
		            ],
		            "getUser": [
		                "[241:8->241:15) getUser",
		            ],
		            "getUserStoreVersion": 0(
		                [
		                    "[38:12->38:13) 0",
		                ],
		            ),
		            "getUsers": [
		                "[245:8->245:16) getUsers",
		            ],
		            "handleLoadCache": [
		                "[224:8->224:23) handleLoadCache",
		            ],
		            "initialize": [
		                "[212:8->212:18) initialize",
		            ],
		            "takeSnapshot": [
		                "[215:8->215:20) takeSnapshot",
		            ],
		            "SYM_CJS_DEFAULT": [
		                "[7:8->7:15) default",
		                "[286:17->286:19) eR",
		                "[211:10->211:12) eR",
		                "[273:8->273:19) constructor",
		            ],
		        },
		    ),
		    "mergeUser": [
		        "[8:8->8:17) mergeUser",
		        "[118:13->118:14) A",
		    ],
		}
		"#);
	}
	#[test]
	fn no_initialize_method() {
		let p = parse!("test_data/wp/stores/store3.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "Z": GuildStore(
		        {
		            "getAllGuildsRoles": [
		                "[218:8->218:25) getAllGuildsRoles",
		            ],
		            "getGeoRestrictedGuilds": [
		                "[53:12->53:14) []",
		            ],
		            "getGuild": [
		                "[199:8->199:16) getGuild",
		            ],
		            "getGuildCount": [
		                "[4:8->4:9) r",
		            ],
		            "getGuildIds": [
		                "[206:8->206:19) getGuildIds",
		            ],
		            "getGuilds": [
		                "[203:8->203:17) getGuilds",
		            ],
		            "getRole": [
		                "[225:8->225:15) getRole",
		            ],
		            "getRoles": [
		                "[221:8->221:16) getRoles",
		            ],
		            "isLoaded": [
		                "[52:12->52:14) !1",
		            ],
		            "SYM_CJS_DEFAULT": [
		                "[6:8->6:9) Z",
		                "[231:16->231:17) U",
		                "[198:10->198:11) U",
		            ],
		        },
		    ),
		}
		"#);
	}

	#[test]
	fn with_getters() {
		let p = parse!("test_data/wp/stores/getter.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "Z": SoundboardOverlayStore(
		        {
		            "enabled": [
		                "[7:12->7:14) !1",
		            ],
		            "keepOpen": [
		                "[8:12->8:14) !1",
		            ],
		            "SYM_CJS_DEFAULT": [
		                "[4:8->4:9) Z",
		                "[24:16->24:17) u",
		                "[9:10->9:11) u",
		            ],
		        },
		    ),
		}
		"#);
	}

	#[test]
	// TODO: libdiscore stores use `define(this, key, prop)` in the constructor a lot
	// which we don't parse
	fn using_libdiscore() {
		let p = parse!("test_data/wp/stores/store-libdiscore-1.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "A": GuildStore(
		        {
		            "getGuildCount": [
		                "[71:8->71:21) getGuildCount",
		            ],
		            "stateWrapper": [
		                "[68:8->68:20) stateWrapper",
		            ],
		            "SYM_CJS_DEFAULT": [
		                "[4:8->4:9) A",
		                "[88:16->88:17) E",
		                "[67:10->67:11) E",
		                "[74:8->74:19) constructor",
		            ],
		        },
		    ),
		}
		"#);
	}

	#[test]
	fn with_static_properties() {
		let p = parse!("test_data/wp/stores/store-static-displayName.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "ASSISTANT_WUMPUS_VOICE_USER": "47835198259242069"(
		        [
		            "[6:8->6:35) ASSISTANT_WUMPUS_VOICE_USER",
		            "[35:12->35:31) \\\"47835198259242069\\\"",
		        ],
		    ),
		    "default": "UserStore"(
		        {
		            "LATEST_SNAPSHOT_VERSION": 1(
		                [
		                    "[594:41->594:42) 1",
		                ],
		            ),
		            "displayName": "UserStore"(
		                [
		                    "[593:29->593:40) \\\"UserStore\\\"",
		                ],
		            ),
		            "filter": [
		                "[710:8->710:14) filter",
		            ],
		            "findByTag": [
		                "[703:8->703:17) findByTag",
		            ],
		            "forEach": [
		                "[698:8->698:15) forEach",
		            ],
		            "getCurrentUser": [
		                "[720:8->720:22) getCurrentUser",
		            ],
		            "getUser": [
		                "[691:8->691:15) getUser",
		            ],
		            "getUserStoreVersion": 0(
		                [
		                    "[34:12->34:13) 0",
		                ],
		            ),
		            "getUsers": [
		                "[695:8->695:16) getUsers",
		            ],
		            "handleLoadCache": [
		                "[676:8->676:23) handleLoadCache",
		            ],
		            "initialize": [
		                "[664:8->664:18) initialize",
		            ],
		            "takeSnapshot": [
		                "[667:8->667:20) takeSnapshot",
		            ],
		            "SYM_CJS_DEFAULT": [
		                "[7:8->7:15) default",
		                "[724:17->724:19) ek",
		                "[592:10->592:12) ek",
		                "[595:8->595:19) constructor",
		            ],
		        },
		    ),
		    "mergeUser": [
		        "[8:8->8:17) mergeUser",
		        "[128:13->128:14) O",
		    ],
		    "transformUser": [
		        "[9:8->9:21) transformUser",
		        "[71:13->71:14) N",
		    ],
		    "users": [
		        "[10:8->10:13) users",
		        "[10:15->10:21) () => ",
		    ],
		}
		"#);
	}

	#[test]
	fn persisted_store() {
		let p = parse!("test_data/wp/stores/persistedStore.js");
		let map = p.dbg_export_map();
		assert_debug_snapshot!(map, @r#"
		{
		    "A": "ThemeStore"(
		        {
		            "displayName": "ThemeStore"(
		                [
		                    "[59:29->59:41) \\\"ThemeStore\\\"",
		                ],
		            ),
		            "getState": [
		                "[78:8->78:16) getState",
		            ],
		            "initialize": [
		                "[70:8->70:18) initialize",
		            ],
		            "migrations": [
		                "[61:28->69:17) [e => {\\n            let t = e.theme;\\n            return \\\"amoled\\\" === t && (t = \\\"midnight\\\"),\\n            {\\n                ...e,\\n                theme: t\\n            }\\n        }\\n        , e => e]",
		            ],
		            "persistKey": "ThemeStore"(
		                [
		                    "[60:28->60:40) \\\"ThemeStore\\\"",
		                ],
		            ),
		            "systemTheme": [
		                "[34:12->35:10) (0,\\n    o.A)()",
		            ],
		            "theme": [
		                "[36:12->36:16) A[I]",
		            ],
		            "themePreferenceForSystemTheme": [
		                "[91:8->91:37) themePreferenceForSystemTheme",
		            ],
		            "SYM_CJS_DEFAULT": [
		                "[6:8->6:9) A",
		                "[95:16->95:17) C",
		                "[58:10->58:11) C",
		            ],
		        },
		    ),
		}
		"#);
	}

	// #[test]
	// fn exported_via_module_exports() {
	// }

	// #[test]
	// fn exported_via_exports() {
	// }
}
