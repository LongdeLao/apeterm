import json
import re
import sys

from pypdf import PdfReader


TRANSACTION_RE = re.compile(
    r"(?P<owner>[A-Z]{2,4})\s+"
    r"(?P<asset>.+?\[[A-Z]+\])\s+"
    r"(?P<type>[A-Z](?: \([^)]+\))?)\s+"
    r"(?P<transaction_date>\d{2}/\d{2}/\d{4})"
    r"(?P<notification_date>\d{2}/\d{2}/\d{4})\s*"
    r"(?P<amount>\$[\d,]+(?:\s*-\s*\$[\d,]+)?)\s+"
    r"(?P<rest>.+?)"
    r"(?=(?:[A-Z]{1,4}\s+.+?\s+[A-Z](?: \([^)]+\))?\s+\d{2}/\d{2}/\d{4}\d{2}/\d{2}/\d{4}\s*\$)|\* For the complete list|I CERTIFY|Digitally Signed:|$)",
    re.DOTALL,
)


def clean_line(value):
    value = value.replace("\x00", "")
    value = re.sub(r"\s+", " ", value).strip()
    return value


def extract_ticker(asset):
    match = re.search(r"\(([A-Z.\-]+)\)\s*\[[A-Z]+\]$", asset)
    return match.group(1) if match else None


def parse_transactions(text):
    transactions = []
    normalized = re.sub(r"\s+", " ", text.replace("\x00", " ")).strip()
    marker = "ID Owner Asset Transaction"
    if marker in normalized:
        normalized = normalized.split(marker, 1)[1]
    normalized = re.sub(
        r"Filing ID #\d+\s+ID Owner Asset Transaction\s+Type\s+Date Notification\s+Date Amount Cap\.\s+Gains > \$200\?",
        " ",
        normalized,
    )
    for match in TRANSACTION_RE.finditer(normalized):
        asset = clean_line(match.group("asset"))
        rest = match.group("rest")
        description_match = re.search(r"D\s*:\s*(.+)", rest)
        transactions.append(
            {
                "owner_code": match.group("owner"),
                "asset_name": asset,
                "ticker": extract_ticker(asset),
                "transaction_type": clean_line(match.group("type")),
                "transaction_date": match.group("transaction_date"),
                "notification_date": match.group("notification_date"),
                "amount_range": clean_line(match.group("amount")),
                "description": clean_line(description_match.group(1)) if description_match else None,
            }
        )
    return transactions


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: house_ptr_extract.py <pdf-path>")

    reader = PdfReader(sys.argv[1])
    text = "\n".join(page.extract_text() or "" for page in reader.pages)
    lines = [clean_line(line) for line in text.splitlines()]
    lines = [line for line in lines if line]

    filed_at = None
    filing_id = None
    for line in lines:
        if line.startswith("Digitally Signed:"):
            match = re.search(r"(\d{2}/\d{2}/\d{4})$", line)
            if match:
                filed_at = match.group(1)
        elif line.startswith("Filing ID #"):
            filing_id = line.split("#", 1)[1].strip()

    transactions = parse_transactions(text)
    print(
        json.dumps(
            {
                "filed_at": filed_at,
                "filing_id": filing_id,
                "transactions": transactions,
            },
            separators=(",", ":"),
        )
    )


if __name__ == "__main__":
    main()
