// PathsumWMC's exact-rational numeric adapter for GPMC. MIT licensed.
// GPMC's counting/search algorithm is unchanged; no MPFR C++ code is used.
#ifndef PATHSUM_GPMC_COMPLEX_H
#define PATHSUM_GPMC_COMPLEX_H
#include <gmpxx.h>
#include <string>
#include <ostream>
class Complex {
public:
    mpq_class real, imag;
    Complex(): real(1), imag(0) {}
    Complex(int r): real(r), imag(0) {}
    Complex(const std::string& r): real(r), imag(0) { real.canonicalize(); }
    Complex(const std::string& r, const std::string& i): real(r), imag(i) { real.canonicalize(); imag.canonicalize(); }
    Complex(const mpq_class& r, const mpq_class& i): real(r), imag(i) {}
    Complex& operator=(int r) { real=r; imag=0; return *this; }
    Complex& operator=(const std::string& r) { real=mpq_class(r); real.canonicalize(); imag=0; return *this; }
    bool operator==(int r) const { return real==r && imag==0; }
    bool operator!=(int r) const { return !(*this==r); }
    bool operator==(const Complex& z) const { return real==z.real && imag==z.imag; }
    bool operator!=(const Complex& z) const { return !(*this==z); }
    Complex operator+(const Complex& z) const { return Complex(real+z.real, imag+z.imag); }
    Complex& operator+=(const Complex& z) { real+=z.real; imag+=z.imag; return *this; }
    Complex operator*(const Complex& z) const { return Complex(real*z.real-imag*z.imag, real*z.imag+imag*z.real); }
    Complex& operator*=(const Complex& z) { *this=(*this)*z; return *this; }
    Complex operator<<(int n) const { mpq_class k(mpz_class(1)<<n); return Complex(real*k,imag*k); }
    friend std::ostream& operator<<(std::ostream& os,const Complex& z) { return os<<z.real<<(z.imag<0?"":"+")<<z.imag<<"i"; }
};
#endif
