from django.urls import path
from django.views.i18n import JavaScriptCatalog, set_language

from shop.views import home

urlpatterns = [
    path("", home, name="home"),
    # The catalog documented at
    # https://docs.djangoproject.com/en/6.1/topics/i18n/translation/#django.views.i18n.JavaScriptCatalog
    # Serves gettext/ngettext/pgettext/npgettext/interpolate/get_format/pluralidx
    # built from locale/<lang>/LC_MESSAGES/djangojs.[po|mo].
    path("jsi18n/", JavaScriptCatalog.as_view(), name="javascript-catalog"),
    path("i18n/setlang/", set_language, name="set_language"),
]
