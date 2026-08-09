class Company {
  u32 revenue 5;
  u32 expenses 3;

  i64 profit::<2> parent.revenue - parent.expenses;

  i64 balance::<3> this + parent.profit;
}
