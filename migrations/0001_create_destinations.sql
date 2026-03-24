CREATE TABLE IF NOT EXISTS destinations (
                                            id INTEGER PRIMARY KEY AUTOINCREMENT,
                                            university_name TEXT NOT NULL,
                                            country TEXT NOT NULL,
                                            location TEXT,
                                            url TEXT,
                                            exchange_type TEXT,
                                            languages TEXT,
                                            description TEXT,
                                            short_name TEXT,
                                            position TEXT
);
