namespace CSharpWip.Project
{
    public sealed class Worker
    {
        private readonly Helper helper = new();

        public string Run(string value)
        {
            return helper.Format(value);
        }
    }

    public sealed class Helper
    {
        public string Format(string value)
        {
            return $"value:{value}";
        }
    }
}
