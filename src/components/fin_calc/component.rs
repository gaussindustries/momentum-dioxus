use dioxus::prelude::*;


//this is more for clever implementation/representation within our calculations 
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy)]
enum Frequency {
    OneTime,
    Daily,
    Weekly,
    Monthly,
    XDays(u32),
    XMonths(u32),
}

impl Frequency {
    pub fn label(&self) -> String {
        match self {
            Frequency::OneTime    => "One-time".into(),
            Frequency::Daily      => "Daily".into(),
            Frequency::Weekly     => "Weekly".into(),
            Frequency::Monthly    => "Monthly".into(),
            Frequency::XDays(n)   => format!("Every {n} days"),
            Frequency::XMonths(n) => format!("Every {n} months"),
        }
    }
}

#[derive(Debug, Clone)]
struct TranchePrimitives {
    created_at: OffsetDateTime,
    frequency: Frequency,
    amount: i64,        // cents, or whatever unit you standardize on
}

impl TranchePrimitives {
    fn new(amount: i64, frequency: Frequency) -> Self {
        Self {
            created_at: OffsetDateTime::now_utc(),
            frequency,
            amount,
        }
    }
}

trait Tranche {
    fn primitives(&self) -> &TranchePrimitives;
    fn primitives_mut(&mut self) -> &mut TranchePrimitives;

    // -------- Getters --------
    fn get_frequency(&self) -> Frequency {
        self.primitives().frequency
    }

    fn get_amount(&self) -> i64 {
        self.primitives().amount
    }

    fn get_created_at(&self) -> OffsetDateTime {
        self.primitives().created_at
    }

    // -------- Setters / mutators --------
    fn set_amount(&mut self, new_amount: i64) {
        self.primitives_mut().amount = new_amount;
    }

    fn add_amount(&mut self, delta: i64) {
        self.primitives_mut().amount += delta;
    }

    fn subtract_amount(&mut self, delta: i64) {
        self.primitives_mut().amount -= delta;
    }

    fn set_frequency(&mut self, freq: Frequency) {
        self.primitives_mut().frequency = freq;
    }
}



#[derive(Debug, Clone)]
enum IncomeKind {
    Hourly {
        hourly_rate_cents: i64,
        avg_hours_per_week: f64,
    },
    Salary {
        yearly_cents: i64,
    },
    Other,
}

impl IncomeKind {
    fn label(&self) -> String {
        match self {
            IncomeKind::Hourly { .. } => "Hourly".into(),
            IncomeKind::Salary { .. } => "Salary".into(),
            IncomeKind::Other        => "Other".into(),
        }
    }
}

#[derive(Debug, Clone)]
struct Income {
    name: String,
    kind: IncomeKind,
    data: TranchePrimitives,
}

impl Income {
    fn new_hourly(name: impl Into<String>, hourly_rate_cents: i64, avg_hours_per_week: f64) -> Self {
        // e.g. convert to approximate monthly amount:
        let weekly = hourly_rate_cents as f64 * avg_hours_per_week;
        let monthly = (weekly * 52.0 / 12.0).round() as i64;

        Self {
            name: name.into(),
            kind: IncomeKind::Hourly {
                hourly_rate_cents,
                avg_hours_per_week,
            },
            data: TranchePrimitives::new(monthly, Frequency::Monthly),
        }
    }

    fn new_salary(name: impl Into<String>, yearly_cents: i64) -> Self {
        let monthly = yearly_cents / 12;
        Self {
            name: name.into(),
            kind: IncomeKind::Salary { yearly_cents },
            data: TranchePrimitives::new(monthly, Frequency::Monthly),
        }
    }
}

impl Tranche for Income {
    fn primitives(&self) -> &TranchePrimitives {
        &self.data
    }

    fn primitives_mut(&mut self) -> &mut TranchePrimitives {
        &mut self.data
    }
}

#[derive(Debug, Clone, Copy)]
enum AssetKind {
    Hard,   // land, metals, etc
    Soft,   // fiat, stocks, etc
}

impl AssetKind {
    fn label(&self) -> &'static str {
        match self {
            AssetKind::Hard => "Hard",
            AssetKind::Soft => "Soft",
        }
    }
}

#[derive(Debug, Clone)]
struct Asset {
    name: String,
    kind: AssetKind,
    data: TranchePrimitives,
}

impl Asset {
    fn new(name: impl Into<String>, kind: AssetKind, amount: i64) -> Self {
        Self {
            name: name.into(),
            kind,
            // Frequency is basically irrelevant; keep it OneTime for now
            data: TranchePrimitives::new(amount, Frequency::OneTime),
        }
    }
}

impl Tranche for Asset {
    fn primitives(&self) -> &TranchePrimitives {
        &self.data
    }

    fn primitives_mut(&mut self) -> &mut TranchePrimitives {
        &mut self.data
    }
}

#[derive(Debug, Clone, Copy)]
enum LiabilityKind {
    Bill,   // recurring bills
    Loan,   // fixed-term loans
    Other,
}

impl LiabilityKind {
    fn label(&self) -> &'static str {
        match self {
            LiabilityKind::Bill => "Bill",
            LiabilityKind::Loan => "Loan",
            LiabilityKind::Other => "Other",
        }
    }
}

#[derive(Debug, Clone)]
struct Liability {
    name: String,
    kind: LiabilityKind,
    data: TranchePrimitives,
}

impl Liability {
    fn new(
        name: impl Into<String>,
        kind: LiabilityKind,
        amount: i64,
        frequency: Frequency,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            data: TranchePrimitives::new(amount, frequency),
        }
    }
}

impl Tranche for Liability {
    fn primitives(&self) -> &TranchePrimitives {
        &self.data
    }

    fn primitives_mut(&mut self) -> &mut TranchePrimitives {
        &mut self.data
    }
}




/*
so essentially what we're trying to do is a generalized approach to adding Tranches
witin our portfolio

common things that can be done to a Tranche
	(once birthed into existence){
		add 
		subtract
		get/set Frequency

	}
	frequency{
		get/set Frequency
		get next date
	
	}

	for each implementation there will be respective functions that are exclusive to each type
	albeit income, assets, liabilities etc


	ensuring seamless experience usage:
	this generalized approach should handle every aspect rather than sphagettification 
	new Tranche(

	
	)


*/

#[component]
pub fn FinCalc(
    // bool with a default: if you don't pass it, it becomes false
    #[props(default)]
    overview: bool,

    // // example: optional string prop
    // #[props(default)]
    // title: Option<String>,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("./style.css") }

		if overview {
			Overview {}
		} else {
			FinCalcDetailed {}
		}
    }
}


/*
	show graph of historical data
	data {
		all:
		assets:
		expenses:
		income:
	
	}

*/
fn format_amount(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.abs();
    let dollars = abs / 100;
    let rem = abs % 100;
    format!("{sign}${dollars}.{rem:02}")
}




fn FinCalcDetailed() -> Element {
    // Incomes, Assets, Liabilities state
    let mut incomes = use_signal(|| {
        vec![
            Income::new_salary("Main Job", 80_000_00),
            Income::new_hourly("Side Gig", 25_00, 10.0),
        ]
    });

    let mut assets = use_signal(|| {
        vec![
            Asset::new("Cash (USD)", AssetKind::Soft, 5_000_00),
            Asset::new("Silver Stack", AssetKind::Hard, 3_000_00),
        ]
    });

    let mut liabilities = use_signal(|| {
        vec![
            Liability::new("Rent", LiabilityKind::Bill, 1_200_00, Frequency::Monthly),
            Liability::new("Car Loan", LiabilityKind::Loan, 350_00, Frequency::Monthly),
        ]
    });
	let mut total_liabilities: i64 = liabilities
					.read()
					.iter()
					.map(|l| l.get_amount())
					.sum();

    rsx! {
        div { class: "flex flex-col gap-8 text-secondary-color border rounded p-6",

            h2 { class: "text-2xl font-bold", "Financial Tranches – Detailed" }

            // Income table
            section {
                h3 { class: "text-xl font-semibold mb-2", "Income" }

                table { class: "w-full text-sm border-collapse",
                    thead {
                        tr {
                            th { class: "border-b border-neutral-700 text-left py-1", "Name" }
                            th { class: "border-b border-neutral-700 text-left py-1", "Kind" }
                            th { class: "border-b border-neutral-700 text-right py-1", "Amount" }
                            th { class: "border-b border-neutral-700 text-left py-1", "Frequency" }
                            th { class: "border-b border-neutral-700 text-center py-1", "Adjust" }
                        }
                    }
                    tbody {
                        for (idx, inc) in incomes.read().iter().enumerate() {
                            tr {
                                key: "{idx}",
                                td { class: "py-1 pr-2", "{inc.name}" }
                                td { class: "py-1 pr-2", "{inc.kind.label()}" }
                                td { class: "py-1 pr-2 text-right",
                                    "{format_amount(inc.get_amount())}"
                                }
                                td { class: "py-1 pr-2", "{inc.get_frequency().label()}" }
                                td { class: "py-1 text-center space-x-1",
                                    button {
                                        class: "px-2 border rounded text-xs",
                                        onclick: {
                                            let mut incomes = incomes.clone();
                                            move |_| {
                                                incomes.write()[idx].add_amount(10_00);
                                            }
                                        },
                                        "+$10"
                                    }
                                    button {
                                        class: "px-2 border rounded text-xs",
                                        onclick: {
                                            let mut incomes = incomes.clone();
                                            move |_| {
                                                incomes.write()[idx].subtract_amount(10_00);
                                            }
                                        },
                                        "-$10"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Assets table
            section {
                h3 { class: "text-xl font-semibold mb-2", "Assets" }

                table { class: "w-full text-sm border-collapse",
                    thead {
                        tr {
                            th { class: "border-b border-neutral-700 text-left py-1", "Name" }
                            th { class: "border-b border-neutral-700 text-left py-1", "Kind" }
                            th { class: "border-b border-neutral-700 text-right py-1", "Value" }
                        }
                    }
                    tbody {
                        for (idx, asset) in assets.read().iter().enumerate() {
                            tr {
                                key: "{idx}",
                                td { class: "py-1 pr-2", "{asset.name}" }
                                td { class: "py-1 pr-2", "{asset.kind.label()}" }
                                td { class: "py-1 pr-2 text-right",
                                    "{format_amount(asset.get_amount())}"
                                }
                            }
                        }
                    }
                }
            }

            // Liabilities table with TOTAL
			section {
				// compute total liabilities
				

				h3 { class: "text-xl font-semibold mb-2",
					"Liabilities – Total: {format_amount(total_liabilities)}"
				}

				table { class: "w-full text-sm border-collapse",
					thead {
						tr {
							th { class: "border-b border-neutral-700 text-left py-1", "Name" }
							th { class: "border-b border-neutral-700 text-left py-1", "Kind" }
							th { class: "border-b border-neutral-700 text-right py-1", "Amount" }
							th { class: "border-b border-neutral-700 text-left py-1", "Frequency" }
						}
					}
					tbody {
						for (idx, liab) in liabilities.read().iter().enumerate() {
							tr {
								key: "{idx}",
								td { class: "py-1 pr-2", "{liab.name}" }
								td { class: "py-1 pr-2", "{liab.kind.label()}" }
								td { class: "py-1 pr-2 text-right",
									"{format_amount(liab.get_amount())}"
								}
								td { class: "py-1 pr-2",
									"{liab.get_frequency().label()}"
								}
							}
						}
					}
				}
			}
        }
    }
}


/*
	could very well be for injecting information quickly rather than
	having to dive into the full detailed version of the program,
	or rather just call how "things are going", the graphical aspect

	snapshot of assets

*/

fn Overview () -> Element {
	rsx!{
        "Overview mode FINCALC"

	}
}


