import type { CustomerRow, ProductRow } from "./types";

// Sample slice of the datablist `customers-100.csv` schema. Mirrors
// the larger `data/customers-sample.csv` shipped in the bundle —
// kept here because the host only serves `ui/*`, not bundle-level
// `data/*`. Three intentionally bad rows at the end exercise the
// data-quality rule preview.
export const SAMPLE_CUSTOMERS: ReadonlyArray<CustomerRow> = [
  { customer_id: "DD37Cf93aecA6Dc", first_name: "Sheryl",   country: "Chile",                 email: "zunigavanessa@smith.info",    subscription_date: "2020-08-24" },
  { customer_id: "1Ef7b82A4CAAD10", first_name: "Preston",  country: "Djibouti",              email: "vmata@colon.com",             subscription_date: "2021-04-23" },
  { customer_id: "5Cef8BFA16c5e3c", first_name: "Linda",    country: "Dominican Republic",    email: "stanleyblackwell@benson.org", subscription_date: "2020-06-02" },
  { customer_id: "053d585Ab6b3159", first_name: "Joanna",   country: "Slovakia",              email: "colinalvarado@miles.net",     subscription_date: "2021-04-17" },
  { customer_id: "EA4d384DfDbBf77", first_name: "Darren",   country: "Pitcairn Islands",      email: "tgates@cantrell.com",         subscription_date: "2021-08-24" },
  { customer_id: "C2dE4dEEc489ae0", first_name: "Sheryl",   country: "Cyprus",                email: "mariokhan@ryan-pope.org",     subscription_date: "2020-01-13" },
  { customer_id: "8C2811a503C7c5a", first_name: "Michelle", country: "Timor-Leste",           email: "mdyer@escobar.net",           subscription_date: "2021-11-08" },
  { customer_id: "CEDec94deE6d69B", first_name: "Jenna",    country: "Vietnam",               email: "mark42@robbins.com",          subscription_date: "2020-11-29" },
  { customer_id: "FFf18C760aA5b27", first_name: "Maxwell",  country: "Malta",                 email: "ehyde@brewer.biz",            subscription_date: "2020-12-19" },
  { customer_id: "BAD-NO-EMAIL-01", first_name: "Casey",    country: "Chile",                 email: "",                            subscription_date: "2021-06-01" },
  { customer_id: "BAD-NO-CNTRY-01", first_name: "Riley",    country: "",                      email: "riley@example.com",           subscription_date: "2021-06-02" },
  { customer_id: "BAD-DATE-001",    first_name: "Morgan",   country: "Slovakia",              email: "morgan@example.com",          subscription_date: "1899-99-99" },
];

export const SAMPLE_PRODUCTS: ReadonlyArray<ProductRow> = [
  { internal_id: "SKU-0001", name: "Slim-Fit Cotton Tee",        brand: "Acme Apparel", category: "Clothing",    price:  19.99, stock: 420, availability: "in_stock" },
  { internal_id: "SKU-0002", name: "Wireless Bluetooth Earbuds", brand: "SoundOrbit",   category: "Electronics", price:  79.00, stock:  12, availability: "low_stock" },
  { internal_id: "SKU-0003", name: "Stainless Travel Mug",       brand: "Hearthware",   category: "Kitchen",     price:  24.50, stock:   0, availability: "out_of_stock" },
  { internal_id: "SKU-0005", name: "Trail Running Shoes",        brand: "Switchback",   category: "Footwear",    price: 129.00, stock:   3, availability: "low_stock" },
  { internal_id: "SKU-0008", name: "Insulated Water Bottle",     brand: "Cascade",      category: "Outdoor",     price:  29.99, stock:   2, availability: "low_stock" },
  { internal_id: "SKU-0010", name: "Cast-Iron Skillet",          brand: "Hearthware",   category: "Kitchen",     price:  44.00, stock:   0, availability: "out_of_stock" },
  { internal_id: "SKU-0013", name: "Espresso Tamper",            brand: "Caffeo",       category: "Kitchen",     price:  55.00, stock:   7, availability: "low_stock" },
  { internal_id: "SKU-0015", name: "USB-C Power Bank",           brand: "Voltcell",     category: "Electronics", price:  89.99, stock:   0, availability: "out_of_stock" },
  { internal_id: "SKU-0016", name: "Down Camp Quilt",            brand: "Switchback",   category: "Outdoor",     price: 189.00, stock:   9, availability: "low_stock" },
  { internal_id: "SKU-0019", name: "Trail Running Vest",         brand: "Switchback",   category: "Fitness",     price:  99.50, stock:   4, availability: "low_stock" },
];
