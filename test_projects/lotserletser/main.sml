class ROOT {
    u64 someRandomValue {
        let useless: u32 = 3;
        let moreUseless: u16 = 723;
        let other: u32 = {
            let useless: u16 = moreUseless;

            useless; // 724
        };

        other = useless + other;
        let useless: u32 = 8;

        other + useless;
    };
}

