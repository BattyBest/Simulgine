class ROOT {
    u64 counter this + 1;

    string hello::<2> {
      if parent.counter > 3
        "counter greater than three"
      else
        "counter less than or equal to three";
    };
}

