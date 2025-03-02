/*
 * I don't quite understand why, but pebble.h includes time.h and then later
 * redefines every relevant time function that is supported. The names clashing
 * upsets the compiler. I'm not sure if there's supported fuctionality in time.h
 * that isn't in pebble.h, but for now override time.h as empty.
 *
 * This header intentionally left blank.
 */
