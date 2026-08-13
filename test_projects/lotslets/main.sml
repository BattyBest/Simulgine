class ROOT {
    u64 someRandomValue {
        let useless: u32 = 3;
        let other: u32 = {
            let useless: u16 = 723;

            useless; // 723
        };

        other = useless + other; // 726
        let useless: u32 = 8;

        other + useless; // 734
    };
}

