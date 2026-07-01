use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString, EnumIter, IntoStaticStr)]
pub enum Key {
    #[strum(serialize = "app.language.english")]
    AppLanguageEnglish,
    #[strum(serialize = "app.language.german")]
    AppLanguageGerman,
    #[strum(serialize = "app.theme.dark")]
    AppThemeDark,
    #[strum(serialize = "app.theme.light")]
    AppThemeLight,
    #[strum(serialize = "app.theme.transparent")]
    AppThemeTransparent,
    #[strum(serialize = "onboarding.prompt.continue")]
    OnboardingPromptContinue,
    #[strum(serialize = "onboarding.title.language")]
    OnboardingTitleLanguage,
    #[strum(serialize = "onboarding.title.theme")]
    OnboardingTitleTheme,
    #[strum(serialize = "panel.title.news")]
    PanelTitleNews,
    #[strum(serialize = "panel.title.watchlist")]
    PanelTitleWatchlist,
    #[strum(serialize = "panel.title.calendar")]
    PanelTitleCalendar,
    #[strum(serialize = "panel.title.notes")]
    PanelTitleNotes,
    #[strum(serialize = "panel.title.picker")]
    PanelTitlePicker,
    #[strum(serialize = "dashboard.help.title")]
    DashboardHelpTitle,
    #[strum(serialize = "dashboard.help.title_split")]
    DashboardHelpTitleSplit,
    #[strum(serialize = "dashboard.help.focus_tab")]
    DashboardHelpFocusTab,
    #[strum(serialize = "dashboard.help.focus_np")]
    DashboardHelpFocusNp,
    #[strum(serialize = "dashboard.help.move_focus")]
    DashboardHelpMoveFocus,
    #[strum(serialize = "dashboard.help.resize")]
    DashboardHelpResize,
    #[strum(serialize = "dashboard.help.change_pane")]
    DashboardHelpChangePane,
    #[strum(serialize = "dashboard.help.split")]
    DashboardHelpSplit,
    #[strum(serialize = "dashboard.help.add_panel")]
    DashboardHelpAddPanel,
    #[strum(serialize = "dashboard.help.search")]
    DashboardHelpSearch,
    #[strum(serialize = "dashboard.help.toggle_locale")]
    DashboardHelpToggleLocale,
    #[strum(serialize = "dashboard.help.drag")]
    DashboardHelpDrag,
    #[strum(serialize = "dashboard.help.click")]
    DashboardHelpClick,
    #[strum(serialize = "dashboard.help.close_panel")]
    DashboardHelpClosePanel,
    #[strum(serialize = "dashboard.help.reset")]
    DashboardHelpReset,
    #[strum(serialize = "dashboard.help.close_help")]
    DashboardHelpCloseHelp,
    #[strum(serialize = "dashboard.help.quit")]
    DashboardHelpQuit,
    #[strum(serialize = "watchlist.section.crypto")]
    WatchlistSectionCrypto,
    #[strum(serialize = "watchlist.section.stocks")]
    WatchlistSectionStocks,
    #[strum(serialize = "watchlist.status.connecting_binance")]
    WatchlistStatusConnectingBinance,
    #[strum(serialize = "watchlist.status.live_quotes_pending")]
    WatchlistStatusLiveQuotesPending,
    #[strum(serialize = "watchlist.market.pre_market")]
    WatchlistMarketPreMarket,
    #[strum(serialize = "watchlist.market.regular")]
    WatchlistMarketRegular,
    #[strum(serialize = "watchlist.market.after_hours")]
    WatchlistMarketAfterHours,
    #[strum(serialize = "calendar.empty")]
    CalendarEmpty,
    #[strum(serialize = "news.empty")]
    NewsEmpty,
    #[strum(serialize = "notes.empty")]
    NotesEmpty,
    #[strum(serialize = "notes.language")]
    NotesLanguage,
    #[strum(serialize = "notes.theme")]
    NotesTheme,
    #[strum(serialize = "notes.session.logged_in")]
    NotesSessionLoggedIn,
    #[strum(serialize = "notes.session.logged_out")]
    NotesSessionLoggedOut,
    #[strum(serialize = "search.filter.stocks")]
    SearchFilterStocks,
    #[strum(serialize = "search.filter.etfs")]
    SearchFilterEtfs,
    #[strum(serialize = "search.help.tab_switches")]
    SearchHelpTabSwitches,
    #[strum(serialize = "search.footer")]
    SearchFooter,
    #[strum(serialize = "search.empty")]
    SearchEmpty,
    #[strum(serialize = "search.header.symbol")]
    SearchHeaderSymbol,
    #[strum(serialize = "search.header.name")]
    SearchHeaderName,
    #[strum(serialize = "search.header.sector_industry")]
    SearchHeaderSectorIndustry,
    #[strum(serialize = "search.status.loaded")]
    SearchStatusLoaded,
    #[strum(serialize = "search.error.details_unavailable")]
    SearchErrorDetailsUnavailable,
    #[strum(serialize = "search.error.database_unavailable")]
    SearchErrorDatabaseUnavailable,
    #[strum(serialize = "details.section.quote")]
    DetailsSectionQuote,
    #[strum(serialize = "details.section.summary")]
    DetailsSectionSummary,
    #[strum(serialize = "details.section.fundamentals")]
    DetailsSectionFundamentals,
    #[strum(serialize = "details.section.details")]
    DetailsSectionDetails,
    #[strum(serialize = "details.status.loading")]
    DetailsStatusLoading,
    #[strum(serialize = "details.status.no_summary")]
    DetailsStatusNoSummary,
    #[strum(serialize = "details.label.current_price")]
    DetailsLabelCurrentPrice,
    #[strum(serialize = "details.label.change")]
    DetailsLabelChange,
    #[strum(serialize = "details.label.previous_close")]
    DetailsLabelPreviousClose,
    #[strum(serialize = "details.label.week_high")]
    DetailsLabelWeekHigh,
    #[strum(serialize = "details.label.week_low")]
    DetailsLabelWeekLow,
    #[strum(serialize = "details.label.market_cap")]
    DetailsLabelMarketCap,
    #[strum(serialize = "details.label.avg_volume")]
    DetailsLabelAvgVolume,
    #[strum(serialize = "details.label.pe_ratio")]
    DetailsLabelPeRatio,
    #[strum(serialize = "details.label.forward_pe")]
    DetailsLabelForwardPe,
    #[strum(serialize = "details.label.dividend_yield")]
    DetailsLabelDividendYield,
    #[strum(serialize = "details.label.earnings")]
    DetailsLabelEarnings,
    #[strum(serialize = "details.label.beta")]
    DetailsLabelBeta,
    #[strum(serialize = "details.label.country")]
    DetailsLabelCountry,
    #[strum(serialize = "details.label.website")]
    DetailsLabelWebsite,
    #[strum(serialize = "details.label.exchange")]
    DetailsLabelExchange,
    #[strum(serialize = "details.label.type")]
    DetailsLabelType,
    #[strum(serialize = "details.label.sector")]
    DetailsLabelSector,
    #[strum(serialize = "details.label.industry")]
    DetailsLabelIndustry,
    #[strum(serialize = "details.label.currency")]
    DetailsLabelCurrency,
    #[strum(serialize = "details.label.active")]
    DetailsLabelActive,
    #[strum(serialize = "details.label.updated")]
    DetailsLabelUpdated,
    #[strum(serialize = "details.value.days")]
    DetailsValueDays,
}
