import lodash from 'lodash';
import moment from 'moment';

const MAX_SAMPLES = 10;
const DEFAULT_NUM_USED = 32;

export interface Entry {
  score: number;
  totalUses: number;
  recentUses: number[];
  frecency: number;
}

export interface PendingUsage {
  key: string;
  timestamp: number;
}

class Frecency<FrecencyItem> {
  dirty: boolean;
  // @ts-expect-error (AUTO)
  _frequently: FrecencyItem[];
  numFrequentlyItems: number;
  maxSamples: number;
  computeBonus: (a: string) => number;
  computeWeight: (a: number) => number;
  lookupKey: (a: string) => FrecencyItem | null | undefined;
  usageHistory: {
    [key: string]: Entry;
  };
  afterCompute: (
    b: {
      [key: string]: Entry;
    },
    a: FrecencyItem[]
  ) => unknown;

  constructor({
    computeBonus,
    computeWeight,
    lookupKey,
    afterCompute,
    numFrequentlyItems = DEFAULT_NUM_USED,
    maxSamples = MAX_SAMPLES,
  }: {
    computeBonus: (key: string) => number;
    computeWeight: (now: number) => number;
    lookupKey: (key: string) => FrecencyItem | null | undefined;
    afterCompute: (b: {[p: string]: Entry}, a: FrecencyItem[]) => unknown;
    numFrequentlyItems?: number;
    maxSamples?: number;
  }) {
    // eslint-disable-line
    this.computeBonus = computeBonus;
    this.computeWeight = computeWeight;
    this.afterCompute = afterCompute;
    this.lookupKey = lookupKey;
    this.usageHistory = {};
    this.frequently = [];
    this.maxSamples = maxSamples;
    this.numFrequentlyItems = numFrequentlyItems;
    this.dirty = false;
  }

  overwriteHistory(entireHistory: {[key: string]: Entry}, pendingUsages?: PendingUsage[]) {
    // Invalidate frecency cache so that we can force a re-computation of the frecency of an entry
    // when we load history from storage.
    this.usageHistory = lodash.mapValues(entireHistory ?? {}, (hist) => ({
      ...hist,
      frecency: -1,
    }));
    pendingUsages?.forEach(({key, timestamp}) => this.track(key, timestamp));

    this.markDirty();
  }

  markDirty() {
    this.dirty = true;
  }

  isDirty(): boolean {
    return this.dirty;
  }

  track(key: string | null | undefined, timestamp?: number) {
    if (key == null) {
      return;
    }
    // It's possible an emoji could be named as 'hasOwnProperty'...
    let entry = Object.prototype.hasOwnProperty.call(this.usageHistory, key) ? this.usageHistory[key] : undefined;
    if (entry == null) {
      entry = {totalUses: 1, recentUses: [timestamp ?? Date.now()], frecency: -1, score: 0};
    } else {
      entry.frecency = -1;
      entry.totalUses += 1;
      if (timestamp == null) {
        entry.recentUses.push(Date.now());
      } else {
        entry.recentUses.push(timestamp);
        entry.recentUses.sort();
      }
      while (entry.recentUses.length > this.maxSamples) {
        entry.recentUses.shift();
      }
    }
    this.usageHistory[key] = entry;
    this.markDirty();
  }

  getEntry(key: string): Entry | null | undefined {
    if (key == null) {
      return null;
    }
    if (this.dirty) {
      this.compute();
    }
    const entry = Object.prototype.hasOwnProperty.call(this.usageHistory, key) ? this.usageHistory[key] : undefined;
    return entry;
  }

  getScore(key: string): number | null | undefined {
    const entry = this.getEntry(key);
    return entry != null ? entry.score : null;
  }

  getFrecency(key: string): number | null | undefined {
    const entry = this.getEntry(key);
    return entry != null ? entry.frecency : null;
  }

  compute() {
    const now = moment();
    lodash.forEach(this.usageHistory, (entry, key) => {
      const {totalUses, recentUses, frecency} = entry;

      if (frecency !== -1) {
        return;
      }
      const bonus = this.computeBonus(key) / 100;
      entry.score = 0;

      lodash.forEach(recentUses, (timestamp, i) => {
        if (i >= this.maxSamples) {
          return false;
        }
        const weight = this.computeWeight(now.diff(moment(timestamp), 'days'));
        entry.score += bonus * weight;
      });

      // If we have a score - we will keep this entry - otherwise the weighting function has
      // decided that this entry should be culled.
      if (entry.score > 0) {
        if (entry.recentUses.length > 0) {
          entry.frecency = Math.ceil(totalUses * (entry.score / recentUses.length));
        }
        this.usageHistory[key] = entry;
      } else {
        delete this.usageHistory[key];
      }
    });

    this.frequently = lodash(this.usageHistory)
      .map((entry, key) => {
        const obj = this.lookupKey(key);
        if (obj == null) {
          return null;
        }
        return [obj, entry.frecency];
      })
      .filter((obj) => obj !== null)
      // @ts-expect-error (AUTO)
      .sortBy(([_, frecency]) => -frecency)
      .map(([obj]) => obj)
      .take(this.numFrequentlyItems)
      .value();

    this.dirty = false;
    this.afterCompute(this.usageHistory, this._frequently);
  }

  get frequently(): FrecencyItem[] {
    if (this.dirty) {
      this.compute();
    }
    return this._frequently;
  }

  set frequently(frequently: FrecencyItem[]) {
    this._frequently = frequently;
  }
}

export default Frecency;
