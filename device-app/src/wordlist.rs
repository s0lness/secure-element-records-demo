//! 256-word SAS list (one word per byte). Two-syllable, phonetically distinct
//! words in the spirit of the PGP word list. Only this app ever renders them:
//! tests and verifier compare screen text between devices, never the list
//! itself, so there is no cross-language copy to keep in sync.

pub const WORDS: [&str; 256] = [
    "acrobat", "adrift", "almond", "amber", "anchor", "antique", "apple", "arena",
    "artist", "asteroid", "atlas", "autumn", "avocado", "azure", "bagpipe", "bamboo",
    "banjo", "barbecue", "beacon", "bedrock", "beehive", "bicycle", "billiard", "bison",
    "blizzard", "blossom", "bluebird", "bonfire", "bookshelf", "breeze", "brioche", "bronze",
    "bubble", "bucket", "butter", "cabaret", "cactus", "camera", "canyon", "caramel",
    "caravan", "carnival", "cascade", "castle", "cavern", "cello", "chapel", "checkers",
    "cherry", "chimney", "chorus", "cinema", "citrus", "clover", "cobalt", "coconut",
    "comet", "compass", "concert", "confetti", "copper", "coral", "cottage", "cricket",
    "crystal", "cyclone", "daisy", "dolphin", "domino", "dragon", "drizzle", "drumbeat",
    "dungeon", "eagle", "echo", "eclipse", "ember", "emerald", "engine", "falcon",
    "feather", "fiddle", "firefly", "flannel", "flamingo", "fortune", "fossil", "fountain",
    "freckle", "frontier", "galaxy", "garden", "gazelle", "geyser", "ginger", "glacier",
    "glitter", "goggles", "gondola", "granite", "grotto", "guitar", "hammock", "harbor",
    "harvest", "hazelnut", "helmet", "hexagon", "hickory", "horizon", "hummingbird", "iceberg",
    "igloo", "indigo", "island", "ivory", "jackal", "jasmine", "jigsaw", "jubilee",
    "juniper", "kayak", "kernel", "kettle", "kiwi", "lagoon", "lantern", "lavender",
    "lemonade", "leopard", "lighthouse", "lilac", "lobster", "locket", "lullaby", "magnet",
    "mango", "marble", "meadow", "melon", "meteor", "mineral", "mirror", "mosaic",
    "mountain", "muffin", "mustang", "nebula", "nectar", "noodle", "nugget", "nutmeg",
    "oasis", "obsidian", "octopus", "olive", "onyx", "opera", "orbit", "orchard",
    "organ", "otter", "oyster", "paddle", "pagoda", "panther", "papaya", "parade",
    "parasol", "peacock", "pebble", "pelican", "pencil", "penguin", "pepper", "petal",
    "phantom", "picnic", "pigeon", "pillow", "pinwheel", "pirate", "pistachio", "planet",
    "plaza", "polka", "pond", "poppy", "prairie", "pretzel", "prism", "pudding",
    "puffin", "pyramid", "quartz", "quiver", "raccoon", "radish", "rainbow", "raspberry",
    "raven", "reindeer", "ribbon", "riddle", "ripple", "rocket", "rooster", "rosemary",
    "ruby", "saffron", "sailboat", "sapphire", "sardine", "satchel", "seahorse", "shadow",
    "shamrock", "sherbet", "silver", "skylark", "sleigh", "snowflake", "sonnet", "sparrow",
    "sphinx", "spiral", "sprout", "squirrel", "stadium", "stallion", "starling", "sundial",
    "sunflower", "syrup", "tadpole", "tangerine", "tavern", "temple", "thimble", "thunder",
    "tiger", "timber", "toffee", "topaz", "trumpet", "tulip", "tundra", "turquoise",
    "twilight", "umbrella", "unicorn", "velvet", "violet", "volcano", "waffle", "walnut",
];
