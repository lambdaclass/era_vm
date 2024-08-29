pragma solidity ^0.8.0;

contract benchmark {
    uint256 value;
    constructor() {
      value = fib(25);
    }

    function get_calculation() public view returns (uint256) {
        return value;
    }

    function fib(uint256 n) internal returns (uint256) {
        if (n <= 1) {
            return n;
        }
        return fib(n - 1) + fib(n - 2);
    }
}
