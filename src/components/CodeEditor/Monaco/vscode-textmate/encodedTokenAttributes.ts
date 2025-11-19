/* eslint-disable @typescript-eslint/no-use-before-define */
/* ---------------------------------------------------------
 * Copyright (C) Microsoft Corporation. All rights reserved.
 *--------------------------------------------------------*/

import { FontStyle } from "./theme";

export type EncodedTokenAttributes = number;

export namespace EncodedTokenAttributes {
    export function toBinaryStr(encodedTokenAttributes: EncodedTokenAttributes): string {
        return encodedTokenAttributes.toString(2).padStart(32, "0");
    }

    export function print(encodedTokenAttributes: EncodedTokenAttributes): void {
        const languageId = EncodedTokenAttributes.getLanguageId(encodedTokenAttributes);
        const tokenType = EncodedTokenAttributes.getTokenType(encodedTokenAttributes);
        const fontStyle = EncodedTokenAttributes.getFontStyle(encodedTokenAttributes);
        const foreground = EncodedTokenAttributes.getForeground(encodedTokenAttributes);
        const background = EncodedTokenAttributes.getBackground(encodedTokenAttributes);

        console.log({
            languageId,
            tokenType,
            fontStyle,
            foreground,
            background,
        });
    }

    export function getLanguageId(encodedTokenAttributes: EncodedTokenAttributes): number {
        return (
            (encodedTokenAttributes & EncodedTokenDataConsts.LANGUAGEID_MASK)
            >>> EncodedTokenDataConsts.LANGUAGEID_OFFSET
        );
    }

    export function getTokenType(encodedTokenAttributes: EncodedTokenAttributes): StandardTokenType {
        return (
            (encodedTokenAttributes & EncodedTokenDataConsts.TOKEN_TYPE_MASK)
            >>> EncodedTokenDataConsts.TOKEN_TYPE_OFFSET
        );
    }

    export function getFontStyle(encodedTokenAttributes: EncodedTokenAttributes): number {
        return (
            (encodedTokenAttributes & EncodedTokenDataConsts.FONT_STYLE_MASK)
            >>> EncodedTokenDataConsts.FONT_STYLE_OFFSET
        );
    }

    export function getForeground(encodedTokenAttributes: EncodedTokenAttributes): number {
        return (
            (encodedTokenAttributes & EncodedTokenDataConsts.FOREGROUND_MASK)
            >>> EncodedTokenDataConsts.FOREGROUND_OFFSET
        );
    }

    export function getBackground(encodedTokenAttributes: EncodedTokenAttributes): number {
        return (
            (encodedTokenAttributes & EncodedTokenDataConsts.BACKGROUND_MASK)
            >>> EncodedTokenDataConsts.BACKGROUND_OFFSET
        );
    }

    /**
     * Updates the fields in `metadata`.
     * A value of `0`, `NotSet` or `null` indicates that the corresponding field should be left as is.
     */
    export function set(
        encodedTokenAttributes: EncodedTokenAttributes,
        languageId: number | 0,
        tokenType: OptionalStandardTokenType | OptionalStandardTokenType.NotSet,
        fontStyle: FontStyle | FontStyle.NotSet,
        foreground: number | 0,
        background: number | 0,
    ): number {
        let _languageId = EncodedTokenAttributes.getLanguageId(encodedTokenAttributes);
        let _tokenType = EncodedTokenAttributes.getTokenType(encodedTokenAttributes);
        let _fontStyle = EncodedTokenAttributes.getFontStyle(encodedTokenAttributes);
        let _foreground = EncodedTokenAttributes.getForeground(encodedTokenAttributes);
        let _background = EncodedTokenAttributes.getBackground(encodedTokenAttributes);

        if (languageId !== 0) {
            _languageId = languageId;
        }
        if (tokenType !== OptionalStandardTokenType.NotSet) {
            _tokenType = fromOptionalTokenType(tokenType);
        }
        if (fontStyle !== FontStyle.NotSet) {
            _fontStyle = fontStyle;
        }
        if (foreground !== 0) {
            _foreground = foreground;
        }
        if (background !== 0) {
            _background = background;
        }

        return (
            ((_languageId << EncodedTokenDataConsts.LANGUAGEID_OFFSET)
              | (_tokenType << EncodedTokenDataConsts.TOKEN_TYPE_OFFSET)
              | (_fontStyle << EncodedTokenDataConsts.FONT_STYLE_OFFSET)
              | (_foreground << EncodedTokenDataConsts.FOREGROUND_OFFSET)
              | (_background << EncodedTokenDataConsts.BACKGROUND_OFFSET))
            >>> 0
        );
    }
}

/**
 * Helpers to manage the "collapsed" metadata of an entire StackElement stack.
 * The following assumptions have been made:
 *  - languageId < 256 => needs 8 bits
 *  - unique color count < 512 => needs 9 bits
 *
 * The binary format is:
 * - -------------------------------------------
 *     3322 2222 2222 1111 1111 1100 0000 0000
 *     1098 7654 3210 9876 5432 1098 7654 3210
 * - -------------------------------------------
 *     xxxx xxxx xxxx xxxx xxxx xxxx xxxx xxxx
 *     bbbb bbbb bfff ffff ffFF FFTT LLLL LLLL
 * - -------------------------------------------
 *  - L = LanguageId (8 bits)
 *  - T = StandardTokenType (2 bits)
 *  - B = Balanced bracket (1 bit)
 *  - F = FontStyle (4 bits)
 *  - f = foreground color (9 bits)
 *  - b = background color (9 bits)
 */
const enum EncodedTokenDataConsts {
    LANGUAGEID_MASK = 0b0000_0000_0000_0000_0000_0000_1111_1111,
    TOKEN_TYPE_MASK = 0b0000_0000_0000_0000_0000_0011_0000_0000,
    FONT_STYLE_MASK = 0b0000_0000_0000_0000_0011_1100_0000_0000,
    FOREGROUND_MASK = 0b0000_0000_0111_1111_1100_0000_0000_0000,
    BACKGROUND_MASK = 0b1111_1111_1000_0000_0000_0000_0000_0000,

    LANGUAGEID_OFFSET = 0,
    TOKEN_TYPE_OFFSET = 8,
    FONT_STYLE_OFFSET = 10,
    FOREGROUND_OFFSET = 14,
    BACKGROUND_OFFSET = 23,
}

export const enum StandardTokenType {
    Other = 0,
    Comment = 1,
    String = 2,
    RegEx = 3,
}

export function toOptionalTokenType(standardType: StandardTokenType): OptionalStandardTokenType {
    return standardType as any as OptionalStandardTokenType;
}

function fromOptionalTokenType(standardType:
  | OptionalStandardTokenType.Other
  | OptionalStandardTokenType.Comment
  | OptionalStandardTokenType.String
  | OptionalStandardTokenType.RegEx): StandardTokenType {
    return standardType as any as StandardTokenType;
}

// Must have the same values as `StandardTokenType`!
export const enum OptionalStandardTokenType {
    Other = 0,
    Comment = 1,
    String = 2,
    RegEx = 3,
    // Indicates that no token type is set.
    NotSet = 8,
}
