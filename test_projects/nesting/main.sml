class Finances {
    double income 5.0;
    double expenses 2.0;
    public double profit::<2> parent.income - parent.expenses;
}

class Company {
    double stockPrice::<2> this + parent.finances.profit / 10.0;

    level Finances finances;
}

class ROOT {
    level Company incorporationsIncorporated;
}

