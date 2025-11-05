use std::collections::{HashMap, HashSet};

use deimos_data::{hash::fnv1, tfx::features::dynamic::SDynamicModelComponent};

pub struct PermutationConfig {
    /// Current configuration of key-value pairs
    pub configuration: HashMap<u32, u32>,

    /// Available values for each key
    keys: HashMap<u32, HashSet<u32>>,

    /// Maps from key-value pairs to permutation index
    pairs_to_permutation: HashMap<Vec<(u32, u32)>, usize>,
}

impl PermutationConfig {
    pub fn from_model(model: &SDynamicModelComponent) -> Option<Self> {
        if model.unk408.is_empty() || model.unk418.is_empty() {
            warn!(
                "TODO: Handle dynamic model permutations without unk408, dont know what to do with these yet"
            );
            return None;
        }
        let mut configuration = HashMap::new();
        for keys1 in &model.unk38 {
            for keys in &keys1.unk8 {
                if keys.value == OPTION_KEY_INVALID {
                    continue;
                }
                configuration.insert(keys.switch_key, keys.value);
            }
        }

        let mut keys = HashMap::new();
        for u0 in &model.unk38 {
            for pair in &u0.unk8 {
                keys.entry(pair.switch_key)
                    .or_insert_with(HashSet::new)
                    .insert(pair.value);
            }
        }

        let mut pairs_to_permutation = HashMap::new();
        for (i, u) in model.unk418.iter().enumerate() {
            if u.unk2 < 0 {
                continue;
            }
            // let start = u.unk6 as usize;
            // let end = start + (u.unk4 as usize);

            // let key_indices = &model.unk408[start..end];
            let mut keys: HashMap<u32, u32> = HashMap::default();
            // for v in key_indices {
            //     let m = &model.unk38[*v as usize];
            //     keys.insert(m.unk8[0].switch_key, m.unk8[0].value);
            // }

            let start = u.unk2 as usize;
            let end = start + (u.unk0 as usize);

            let key_indices = &model.unk408[start..end];
            for v in key_indices {
                let m = &model.unk38[*v as usize];
                keys.insert(m.unk8[0].switch_key, m.unk8[0].value);
            }

            let mut pair = keys.into_iter().collect::<Vec<(u32, u32)>>();
            pair.sort_by_key(|(k, _)| *k);
            pairs_to_permutation.insert(pair, i);
        }

        Some(Self {
            configuration,
            keys,
            pairs_to_permutation,
        })
    }

    pub fn iter_keys(&self) -> impl Iterator<Item = (u32, &HashSet<u32>)> {
        self.keys.iter().map(|(k, v)| (*k, v))
    }

    pub fn for_each_key_mut<F>(&mut self, mut f: F)
    where
        // key_hash, available_values, current_value
        F: FnMut(u32, &HashSet<u32>, &mut u32),
    {
        for (key, available_values) in &self.keys {
            if let Some(current_value) = self.configuration.get_mut(key) {
                f(*key, available_values, current_value);
            }
        }
    }

    pub fn calculate_permutation_index(&self) -> Option<usize> {
        let mut key_vals: Vec<(u32, u32)> =
            self.configuration.iter().map(|(k, v)| (*k, *v)).collect();
        key_vals.sort_by_key(|(k, _)| *k);
        self.pairs_to_permutation.get(&key_vals).copied()
    }
}

#[rustfmt::skip]
const FNV_NAMES: &[&str] = &[
    "00", "01", "02", "03", "04", "05", "06", "07", "08", "1", "10", "1x1", "2", "2x2", "3", "4", "4x4", "5", "6", "7", "8", "9", "a",
    "accent", "activate", "activated", "advanced", "alien", "ammo", "ammo_type", "arm_cannon", "assault", "assault_shield", "b", "backpack",
    "base", "baseplate", "battery", "black", "blue", "body", "boss", "box", "brightness", "bullets", "c", "ceiling", "color", "consumable",
    "content", "core", "cyan", "d", "damage", "deactivate", "decal", "decals", "default", "deployable", "destroyed", "device_power", "dim",
    "eggs", "elite", "empty", "equipment", "flicker", "flux", "frost", "full", "fuse", "fusion", "garbage", "gear", "gender", "ghost",
    "grayscale", "green", "grenadier", "grime", "hair", "head", "heavy", "helmet", "high", "hologram", "hull", "implant", "interference",
    "interior", "inverted", "jump_over", "large", "left_open", "light", "light color", "light intensity", "light1", "light2", "light3",
    "light4", "light5", "loot_container", "main", "major", "male", "max", "medium", "mid", "min", "miniboss", "minor", "mips", "mold",
    "mount", "none", "occupied", "off", "on", "open", "orange", "permutation", "pink", "power_on", "primed", "prop", "pulse", "recruit",
    "red", "region", "right_open", "scout_rifle", "shotgun", "size", "skin", "smoothness", "sniper", "standard", "standing", "state",
    "struggle", "tabletop", "teal", "terminal", "to_be_set", "type", "undamaged", "unit_type", "usage", "uv", "voucher", "wall", "waterline",
    "weapon", "weapon_mod", "weapon_type", "white", "worker", "yellow", "zero",
];

pub const OPTION_KEY_INVALID: u32 = 0x871AC0EA;

const FNV_NAME_GUESSES: &[(u32, &str)] = &[
    (OPTION_KEY_INVALID, "<invalid>"),
    (0x20809827, "dark gray*"),
    (0xCFA916D2, "light gray*"),
    (0x9D102655, "white*"),
    (0x78532C1A, "olive*"),
    (0xDFF5552A, "dark green*"),
    (0x1023B2D3, "color*"),
];

fn find_fnv_name(hash: u32) -> Option<&'static str> {
    if let Some(s) = FNV_NAMES
        .iter()
        .find(|&&name| fnv1(name.as_bytes()) == hash)
        .copied()
        .map(|v| v as _)
    {
        Some(s)
    } else {
        FNV_NAME_GUESSES
            .iter()
            .find(|&&(h, _)| h == hash)
            .map(|&(_, name)| name)
    }
}

pub fn find_kv_name(hash: u32) -> Option<&'static str> {
    find_fnv_name(hash)
}

pub fn find_kv_name_or_default(hash: u32) -> String {
    find_kv_name(hash)
        .map(|v| v.to_string())
        .unwrap_or_else(|| format!("unknown_{hash:08X}"))
}
