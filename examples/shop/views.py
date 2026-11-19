from django.shortcuts import render


def home(request):
    """Demo page: server-side strings (django domain) + the JS catalog."""
    return render(request, "shop/home.html")
